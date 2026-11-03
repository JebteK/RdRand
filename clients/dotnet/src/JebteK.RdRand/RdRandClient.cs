using System;
using System.Globalization;
using System.Net.Http;
using System.Text;
using System.Threading;
using System.Threading.Tasks;

namespace JebteK.RdRand
{
    /// <summary>Where randomness may come from.</summary>
    public enum RdRandMode
    {
        /// <summary>The CPU's generator if it has one, else the rdrand.com API (also on a hardware failure).</summary>
        Auto,
        /// <summary>Only the CPU's generator; fail if there is none.</summary>
        Hardware,
        /// <summary>Only the rdrand.com API.</summary>
        Service,
    }

    /// <summary>Where a result came from.</summary>
    public enum RdRandSource
    {
        /// <summary>x86_64 RDRAND (Intel, AMD).</summary>
        Rdrand,
        /// <summary>ARMv8.5 RNDR (e.g. AWS Graviton3/4).</summary>
        Rndr,
        /// <summary>The rdrand.com API.</summary>
        Service,
    }

    /// <summary>The hardware generator or the rdrand.com API failed.</summary>
    public sealed class RdRandException : Exception
    {
        /// <summary>Creates the exception.</summary>
        public RdRandException(string message, Exception? inner = null) : base(message, inner) { }
    }

    /// <summary>
    /// rdrand.com client: hardware random numbers from the local CPU (RDRAND on
    /// Intel/AMD, RNDR on ARMv8.5+ such as AWS Graviton3), falling back to the
    /// rdrand.com API. Results have the API's formats whichever source made them.
    /// Thread-safe; reuse one instance.
    /// </summary>
    public sealed class RdRandClient : IDisposable
    {
        /// <summary>The API's base URL.</summary>
        public const string DefaultBaseUrl = "https://rdrand.com/API/";

        /// <summary>The 63 characters keys are drawn from, each equally likely.</summary>
        public const string KeyAlphabet = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz";

        private const string Version = "2.0.0";

        private readonly string _baseUrl;
        private readonly HttpClient _http;
        private readonly bool _ownsHttp;

        /// <param name="mode">Where randomness may come from.</param>
        /// <param name="baseUrl">The API's base URL.</param>
        /// <param name="httpClient">An HttpClient to use for API calls (not disposed by this client).</param>
        public RdRandClient(RdRandMode mode = RdRandMode.Auto, string baseUrl = DefaultBaseUrl, HttpClient? httpClient = null)
        {
            Mode = mode;
            _baseUrl = baseUrl.TrimEnd('/');
            _ownsHttp = httpClient == null;
            _http = httpClient ?? new HttpClient { Timeout = TimeSpan.FromSeconds(60) };
        }

        /// <summary>Where randomness may come from.</summary>
        public RdRandMode Mode { get; }

        /// <summary>The CPU's hardware generator, or null if it has none.</summary>
        public static RdRandSource? HardwareSource => NativeMethods.Source;

        /// <summary>Where this client's results come from.</summary>
        public RdRandSource Source => Mode == RdRandMode.Service ? RdRandSource.Service : HardwareSource ?? RdRandSource.Service;

        /// <summary><paramref name="length"/> characters, each uniform over 0-9 A-Z _ a-z.</summary>
        public Task<string> GenerateKeyAsync(int length, CancellationToken cancellationToken = default)
        {
            CheckLength(length);
            return WithFallbackAsync(() => KeyFromHardware(length), async () =>
            {
                var s = await PostAsync("GenerateKey", length, cancellationToken).ConfigureAwait(false);
                if (s.Length != length || !IsKey(s))
                {
                    throw new RdRandException($"malformed key from the API ({s.Length} chars)");
                }
                return s;
            });
        }

        /// <summary><paramref name="length"/> random bytes.</summary>
        public Task<byte[]> GenerateBytesAsync(int length, CancellationToken cancellationToken = default)
        {
            CheckLength(length);
            return WithFallbackAsync(() =>
            {
                var buf = new byte[length];
                return NativeMethods.Fill(buf) ? buf : null;
            }, async () =>
            {
                var s = await PostAsync("GenerateBytes", length, cancellationToken).ConfigureAwait(false);
                byte[] raw;
                try
                {
                    raw = Convert.FromBase64String(s.Trim());
                }
                catch (FormatException e)
                {
                    throw new RdRandException("invalid base64 from the API", e);
                }
                if (raw.Length != length)
                {
                    throw new RdRandException($"expected {length} bytes from the API, got {raw.Length}");
                }
                return raw;
            });
        }

        /// <summary>A random integer from 0 to 2^31 - 1 (the API's range).</summary>
        public async Task<uint> GenerateUIntAsync(CancellationToken cancellationToken = default)
        {
            var boxed = await WithFallbackAsync<object>(() =>
            {
                var b = new byte[4];
                return NativeMethods.Fill(b) ? (object)(BitConverter.ToUInt32(LittleEndian(b), 0) & 0x7fffffffu) : null;
            }, async () =>
            {
                var s = (await PostAsync("GenerateUInt", null, cancellationToken).ConfigureAwait(false)).Trim();
                if (!uint.TryParse(s, NumberStyles.None, CultureInfo.InvariantCulture, out var n))
                {
                    throw new RdRandException($"not an integer from the API: {(s.Length > 40 ? s.Substring(0, 40) : s)}");
                }
                return n;
            }).ConfigureAwait(false);
            return (uint)boxed;
        }

        /// <summary>Blocking <see cref="GenerateKeyAsync"/>.</summary>
        public string GenerateKey(int length) => GenerateKeyAsync(length).GetAwaiter().GetResult();

        /// <summary>Blocking <see cref="GenerateBytesAsync"/>.</summary>
        public byte[] GenerateBytes(int length) => GenerateBytesAsync(length).GetAwaiter().GetResult();

        /// <summary>Blocking <see cref="GenerateUIntAsync"/>.</summary>
        public uint GenerateUInt() => GenerateUIntAsync().GetAwaiter().GetResult();

        /// <inheritdoc />
        public void Dispose()
        {
            if (_ownsHttp)
            {
                _http.Dispose();
            }
        }

        private async Task<T> WithFallbackAsync<T>(Func<T?> hardware, Func<Task<T>> remote) where T : class
        {
            if (Mode != RdRandMode.Service)
            {
                var v = hardware();
                if (v != null)
                {
                    return v;
                }
                if (Mode == RdRandMode.Hardware)
                {
                    throw new RdRandException(HardwareSource != null
                        ? "the hardware random number generator failed to deliver"
                        : "this CPU has no working hardware random number generator");
                }
            }
            return await remote().ConfigureAwait(false);
        }

        /// <summary>Uniform over the 63-character alphabet: bytes 252..255 are rejected.</summary>
        private static string? KeyFromHardware(int length)
        {
            var sb = new StringBuilder(length);
            while (sb.Length < length)
            {
                var want = length - sb.Length;
                var buf = new byte[Math.Min(want + want / 32 + 8, 65536)];
                if (!NativeMethods.Fill(buf))
                {
                    return null;
                }
                foreach (var b in buf)
                {
                    if (b < 252 && sb.Length < length)
                    {
                        sb.Append(KeyAlphabet[b % 63]);
                    }
                }
            }
            return sb.ToString();
        }

        private async Task<string> PostAsync(string action, int? length, CancellationToken cancellationToken)
        {
            // An explicit (possibly empty) body so Content-Length is always sent:
            // the legacy IIS servers reject a body-less POST with 411.
            HttpContent content = length.HasValue
                ? new StringContent("length=" + length.Value.ToString(CultureInfo.InvariantCulture), Encoding.ASCII, "application/x-www-form-urlencoded")
                : new ByteArrayContent(Array.Empty<byte>());
            using (var request = new HttpRequestMessage(HttpMethod.Post, _baseUrl + "/" + action) { Content = content })
            {
                request.Headers.UserAgent.ParseAdd("rdrand-client-dotnet/" + Version);
                HttpResponseMessage response;
                try
                {
                    response = await _http.SendAsync(request, cancellationToken).ConfigureAwait(false);
                }
                catch (HttpRequestException e)
                {
                    throw new RdRandException("rdrand.com API: " + e.Message, e);
                }
                catch (TaskCanceledException e) when (!cancellationToken.IsCancellationRequested)
                {
                    throw new RdRandException("rdrand.com API: timed out", e);
                }
                using (response)
                {
                    var body = await response.Content.ReadAsStringAsync().ConfigureAwait(false);
                    if ((int)response.StatusCode != 200)
                    {
                        throw new RdRandException($"rdrand.com API: HTTP {(int)response.StatusCode}");
                    }
                    return body;
                }
            }
        }

        private static bool IsKey(string s)
        {
            foreach (var c in s)
            {
                if (KeyAlphabet.IndexOf(c) < 0)
                {
                    return false;
                }
            }
            return true;
        }

        private static byte[] LittleEndian(byte[] b)
        {
            if (!BitConverter.IsLittleEndian)
            {
                Array.Reverse(b);
            }
            return b;
        }

        private static void CheckLength(int length)
        {
            if (length < 0)
            {
                throw new ArgumentOutOfRangeException(nameof(length), "length must not be negative");
            }
        }
    }
}
