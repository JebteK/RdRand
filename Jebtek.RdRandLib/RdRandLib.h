// RdRand.h

#pragma once

using namespace System;

namespace Jebtek {

	public ref class RdRandLib
	{
	private:
		static bool isGeneratorAvailable = false;
	public:
		//returns true if the random number generator is available
		static bool GeneratorAvailable();

		//Returns a 32-bit random number
		static unsigned int Random32();
	};
}
