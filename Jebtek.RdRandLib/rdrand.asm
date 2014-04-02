; Execute rdrand in assembly 
.code 
PUBLIC __RdRand 
__RdRand PROC 
	rdrand eax
	jnz success
	mov eax, -1
	ret
success:
	ret
__RdRand ENDP 
End
