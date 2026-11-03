package com.jebtek.rdrand

import java.nio.file.Files
import java.nio.file.StandardCopyOption

/**
 * JNI bindings to the bundled hardware RNG library (`rdrand_native`), shipped
 * in the jar for Linux, Windows and macOS on x64 and arm64. It is extracted to
 * a temporary file and loaded on first use; if that fails the client uses the
 * rdrand.com API instead.
 */
internal object NativeRng {
    /** 0 none, 1 RDRAND, 2 RNDR. */
    @JvmStatic
    external fun source(): Int

    /** 0 filled, -1 no hardware generator, -2 hardware failure. */
    @JvmStatic
    external fun fill(buf: ByteArray): Int

    val loaded: Boolean by lazy { load() }

    private fun platform(): String? {
        val os = System.getProperty("os.name").lowercase()
        val arch = when (System.getProperty("os.arch").lowercase()) {
            "amd64", "x86_64" -> "x64"
            "aarch64", "arm64" -> "arm64"
            else -> return null
        }
        return when {
            os.startsWith("linux") -> "linux-$arch"
            os.startsWith("mac") || os.startsWith("darwin") -> "darwin-$arch"
            os.startsWith("windows") -> "win32-$arch"
            else -> null
        }
    }

    private fun load(): Boolean {
        val platform = platform() ?: return false
        val name = when {
            platform.startsWith("win32") -> "rdrand_native.dll"
            platform.startsWith("darwin") -> "librdrand_native.dylib"
            else -> "librdrand_native.so"
        }
        val resource = NativeRng::class.java.getResourceAsStream("native/$platform/$name") ?: return false
        return try {
            val dir = Files.createTempDirectory("rdrand-native")
            val file = dir.resolve(name)
            resource.use { Files.copy(it, file, StandardCopyOption.REPLACE_EXISTING) }
            file.toFile().deleteOnExit()
            dir.toFile().deleteOnExit()
            System.load(file.toAbsolutePath().toString())
            true
        } catch (e: Throwable) {
            // UnsatisfiedLinkError, IOException, SecurityException: use the API.
            false
        }
    }
}
