RdRand
======

Library to use Intel's Secure Key (rdrand instruction) in .NET

You can include the Jebtek.RdRand library in any .NET project and use the static methods in the RdRandom class to access 
the random number generator. Helper functions have been created to generate random strings, API keys etc. as well.

This code will only execute on the latest Intel Ivy Bridge and higher CPUs.

The "Versions" directory has all the release builds in it. The latest release build of the code is suffixed with "-Latest". Jebtek.RdRandLib is the source library that outputs random numbers by executing the "rdrand" instruction. Jebtek.RdRand is a wrapper around the library that provides higher level functions, such as generating random strings, etc.

If you need access to a true RNG but don't have the latest Intel processors, take a look at https://rdrand.com

License (MIT)
======

Copyright © 2014 JebteK, LLC

Permission is hereby granted, free of charge, to any person obtaining a copy of this software and associated documentation files (the "Software"), to deal in the Software without restriction, including without limitation the rights to use, copy, modify, merge, publish, distribute, sublicense, and/or sell copies of the Software, and to permit persons to whom the Software is furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM, OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE SOFTWARE.
