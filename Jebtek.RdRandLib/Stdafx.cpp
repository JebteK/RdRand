// stdafx.cpp : source file that includes just the standard includes
// RdRand.pch will be the pre-compiled header
// stdafx.obj will contain the pre-compiled type information

#include "stdafx.h"

#include <intrin.h>
#include <string.h>

extern "C" unsigned int __RdRand(); 

#pragma managed(push, off)
bool __GeneratorAvailable()
{
	int info[4] = {-1};

	/* Are we on an Intel processor? */

	__cpuid(info, /*feature bits*/0);

	if ( memcmp((void *) &info[1], (void *) "Genu", 4) != 0 ||
		memcmp((void *) &info[3], (void *) "ineI", 4) != 0 ||
		memcmp((void *) &info[2], (void *) "ntel", 4) != 0 ) 
	{
		return 0;
	}
	
	/* Do we have RDRAND? */

	 __cpuid(info, /*feature bits*/1);

	 int ecx = info[2];
	 if ((ecx & RDRAND_MASK) == RDRAND_MASK)
		 return 1;
	 else
		 return 0;

	return false;
}
#pragma managed(pop)

#pragma managed(push, off)
unsigned int __RandomLooper32()
{
	/*
	__asm
   {
		rdrand eax
		jnz success
		mov eax, -1
success:
   }*/
	return __RdRand();
}
#pragma managed(pop)

#pragma managed(push, off)
unsigned int __Random32()
{
	for (int i = 0; i < 10; i++)
	{
		int r = __RandomLooper32();
		if (r > 0)
			return r;
	}

	return 0;
}
#pragma managed(pop)