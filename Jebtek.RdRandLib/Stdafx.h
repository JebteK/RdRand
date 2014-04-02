// stdafx.h : include file for standard system include files,
// or project specific include files that are used frequently,
// but are changed infrequently

#pragma once

#define RDRAND_MASK	0x40000000

#define GENERATOR_UNAVAILABLE -1

#define RETRY_LIMIT 10

bool __GeneratorAvailable();
unsigned int __RandomLooper32();
unsigned int __Random32();