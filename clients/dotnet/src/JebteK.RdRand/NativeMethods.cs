using System;
using System.Runtime.InteropServices;

namespace JebteK.RdRand
{
    /// <summary>
    /// P/Invoke into the bundled hardware RNG library (rdrand_native), shipped in
    /// the package's runtimes/ folder for linux-x64, linux-arm64, win-x64,
    /// osx-x64 and osx-arm64.
    /// </summary>
    internal static class NativeMethods
    {
        private const string Lib = "rdrand_native";

        /// <summary>0 none, 1 RDRAND, 2 RNDR.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        private static extern int rdrand_native_source();

        /// <summary>0 filled, -1 no hardware generator, -2 hardware failure.</summary>
        [DllImport(Lib, CallingConvention = CallingConvention.Cdecl)]
        private static extern int rdrand_native_fill([Out] byte[] buf, UIntPtr len);

        private static readonly Lazy<int> SourceCode = new Lazy<int>(() =>
        {
            try
            {
                return rdrand_native_source();
            }
            catch (Exception e) when (e is DllNotFoundException || e is EntryPointNotFoundException || e is BadImageFormatException)
            {
                // No library for this platform: the API is used instead.
                return 0;
            }
        });

        internal static RdRandSource? Source => SourceCode.Value switch
        {
            1 => RdRandSource.Rdrand,
            2 => RdRandSource.Rndr,
            _ => null,
        };

        internal static bool Fill(byte[] buf) =>
            Source != null && (buf.Length == 0 || rdrand_native_fill(buf, (UIntPtr)buf.Length) == 0);
    }
}
