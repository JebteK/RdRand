RdRand
======

Library to use Intel's Secure Key (rdrand instruction) in .NET

You can include the Jebtek.RdRand library in any .NET project and use the static methods in the RdRandom class to access 
the random number generator. Helper functions have been created to generate random strings, API keys etc. as well.

This code will only execute on the latest Intel Ivy Bridge and higher CPUs.

If you need access to a true RNG but don't have the latest Intel processors, you can use a service that has been setup 
at https://rdrand.com