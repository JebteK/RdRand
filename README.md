RdRand
======

Library to use Intel's Secure Key (rdrand instruction) in .NET

You can include the Jebtek.RdRand library in any .NET project and use the static methods in the RdRandom class to access 
the random number generator. Helper functions have been created to generate random strings, API keys etc. as well.

This code will only execute on the latest Intel Ivy Bridge and higher CPUs.

The "Versions" directory has all the release builds in it. The latest release build of the code is suffixed with "-Latest". Jebtek.RdRandLib is the source library that outputs random numbers by executing the "rdrand" instruction. Jebtek.RdRand is a wrapper around the library that provides higher level functions, such as generating random strings, etc.

If you need access to a true RNG but don't have the latest Intel processors, take a look at https://rdrand.com
