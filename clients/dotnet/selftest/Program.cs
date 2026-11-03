// Self-test for the JebteK.RdRand package. Set RDRAND_TEST_URL (e.g.
// https://rdrand.com/API/) to also exercise the API.
using System.Runtime.InteropServices;
using JebteK.RdRand;

var failures = 0;
void Check(string name, bool ok, string detail = "")
{
    Console.WriteLine($"  [{(ok ? "PASS" : "FAIL")}] {name}{(detail.Length > 0 ? ": " + detail : "")}");
    if (!ok) failures++;
}
bool IsKey(string s, int n) => s.Length == n && s.All(c => RdRandClient.KeyAlphabet.Contains(c));

var hw = RdRandClient.HardwareSource;
Console.WriteLine($"hardware generator: {hw?.ToString().ToLowerInvariant() ?? "none"} ({RuntimeInformation.RuntimeIdentifier}, {RuntimeInformation.FrameworkDescription})");
using (var auto0 = new RdRandClient())
{
    Check("auto source", auto0.Source == (hw ?? RdRandSource.Service), auto0.Source.ToString());
}

using (var hwRng = new RdRandClient(RdRandMode.Hardware))
{
    if (hw == null)
    {
        Check("hardware mode refuses", await Throws<RdRandException>(() => hwRng.GenerateKeyAsync(4)));
    }
    else
    {
        Check("hardware key", IsKey(await hwRng.GenerateKeyAsync(5000), 5000));
        Check("hardware empty key", (await hwRng.GenerateKeyAsync(0)).Length == 0);
        var bytes = await hwRng.GenerateBytesAsync(1 << 20);
        var zeroWords = 0;
        for (var i = 0; i < bytes.Length; i += 4) if (BitConverter.ToUInt32(bytes, i) == 0) zeroWords++;
        Check("hardware 1 MiB bytes, no zero words", bytes.Length == 1 << 20 && zeroWords == 0, $"{zeroWords} zero words");
        var n = await hwRng.GenerateUIntAsync();
        Check("hardware uint in range", n < 1u << 31, n.ToString());
        Check("hardware sync API", IsKey(hwRng.GenerateKey(16), 16) && hwRng.GenerateBytes(8).Length == 8);
        var key = await hwRng.GenerateKeyAsync(252_000);
        var counts = key.GroupBy(c => c).Select(g => g.Count()).ToList();
        var expected = key.Length / 63.0;
        var chi2 = counts.Sum(c => (c - expected) * (c - expected) / expected);
        Check("hardware key uniform", counts.Count == 63 && chi2 < 120, $"chi2={chi2:F1}");
    }
}

var url = Environment.GetEnvironmentVariable("RDRAND_TEST_URL");
if (url == null)
{
    Console.WriteLine("  [SKIP] service mode: RDRAND_TEST_URL not set");
}
else
{
    using var svc = new RdRandClient(RdRandMode.Service, url);
    Check("service source", svc.Source == RdRandSource.Service);
    Check("service key", IsKey(await svc.GenerateKeyAsync(100), 100));
    Check("service empty key", (await svc.GenerateKeyAsync(0)).Length == 0);
    Check("service bytes", (await svc.GenerateBytesAsync(1000)).Length == 1000);
    Check("service uint", await svc.GenerateUIntAsync() < 1u << 31);
    Check("service sync uint", svc.GenerateUInt() < 1u << 31);
    using var bad = new RdRandClient(RdRandMode.Service, url + "Nope/");
    Check("service error surfaces", await Throws<RdRandException>(() => bad.GenerateUIntAsync()));
}

using (var auto = new RdRandClient(baseUrl: url ?? RdRandClient.DefaultBaseUrl))
{
    Check("auto key", IsKey(await auto.GenerateKeyAsync(32), 32));
    Check("negative length rejected", await Throws<ArgumentOutOfRangeException>(() => auto.GenerateBytesAsync(-1)));
}

Console.WriteLine(failures == 0 ? "all checks passed" : $"{failures} check(s) FAILED");
return failures == 0 ? 0 : 1;

static async Task<bool> Throws<T>(Func<Task> f) where T : Exception
{
    try { await f(); return false; }
    catch (T) { return true; }
}
