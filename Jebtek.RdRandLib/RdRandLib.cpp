// This is the main DLL file.

#include "stdafx.h"

#include <intrin.h>
#include <string.h>

#include "RdRandLib.h"

bool Jebtek::RdRandLib::GeneratorAvailable()
{
	//only check if we determined that the generator is not available
	if (!isGeneratorAvailable)
		isGeneratorAvailable = __GeneratorAvailable();

	return isGeneratorAvailable;
}

//Returns 0 if a random number was not generated
unsigned int Jebtek::RdRandLib::Random32()
{
	if (!isGeneratorAvailable)
		return GENERATOR_UNAVAILABLE;

	int r = __Random32();

	return r;
}