package com.jebtek.rdrand

import java.io.IOException
import java.net.URI
import java.net.http.HttpClient
import java.net.http.HttpRequest
import java.net.http.HttpResponse
import java.time.Duration
import java.util.Base64

/** Where randomness may come from. */
enum class Mode {
    /** The CPU's generator if it has one, else the rdrand.com API (also on a hardware failure). */
    AUTO,

    /** Only the CPU's generator; fail if there is none. */
    HARDWARE,

    /** Only the rdrand.com API. */
    SERVICE,
}

/** Where a result came from. */
enum class Source(val id: String) {
    /** x86_64 RDRAND (Intel, AMD). */
    RDRAND("rdrand"),

    /** ARMv8.5 RNDR (e.g. AWS Graviton3/4). */
    RNDR("rndr"),

    /** The rdrand.com API. */
    SERVICE("service"),
}

class RdRandException(message: String, cause: Throwable? = null) : RuntimeException(message, cause)

/**
 * rdrand.com client: hardware random numbers from the local CPU (RDRAND on
 * Intel/AMD, RNDR on ARMv8.5+ such as AWS Graviton3), falling back to the
 * rdrand.com API. Results have the API's formats whichever source made them.
 * Thread-safe; calls block (hardware calls take microseconds).
 *
 * ```
 * val rng = RdRand()
 * val key = rng.generateKey(32)      // 32 chars from 0-9 A-Z _ a-z
 * val bytes = rng.generateBytes(16)  // 16 random bytes
 * val n = rng.generateUInt()         // 0 .. Int.MAX_VALUE
 * ```
 */
class RdRand @JvmOverloads constructor(
    val mode: Mode = Mode.AUTO,
    baseUrl: String = DEFAULT_BASE_URL,
    private val timeout: Duration = Duration.ofSeconds(60),
) {
    private val baseUrl = baseUrl.trimEnd('/')
    // HTTP/1.1 so an empty POST carries Content-Length: 0. Over HTTP/2 Java
    // omits it, and the legacy IIS servers reject that with 411.
    private val http: HttpClient by lazy {
        HttpClient.newBuilder().version(HttpClient.Version.HTTP_1_1).connectTimeout(timeout).build()
    }

    companion object {
        const val DEFAULT_BASE_URL = "https://rdrand.com/API/"
        const val KEY_ALPHABET = "0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZ_abcdefghijklmnopqrstuvwxyz"
        const val VERSION = "2.0.0"

        /** The CPU's hardware generator, or null if it has none. */
        @JvmStatic
        fun hardwareSource(): Source? {
            if (!NativeRng.loaded) return null
            return when (NativeRng.source()) {
                1 -> Source.RDRAND
                2 -> Source.RNDR
                else -> null
            }
        }

        private fun hardwareFill(buf: ByteArray): Boolean =
            hardwareSource() != null && NativeRng.fill(buf) == 0
    }

    /** Where results come from. */
    val source: Source
        get() = if (mode == Mode.SERVICE) Source.SERVICE else hardwareSource() ?: Source.SERVICE

    /** [length] characters, each uniform over 0-9 A-Z _ a-z. */
    fun generateKey(length: Int): String {
        requireLength(length)
        return withFallback({ keyFromHardware(length) }) {
            val s = post("GenerateKey", length)
            if (s.length != length || s.any { it !in KEY_ALPHABET }) {
                throw RdRandException("malformed key from the API (${s.length} chars)")
            }
            s
        }
    }

    /** [length] random bytes. */
    fun generateBytes(length: Int): ByteArray {
        requireLength(length)
        return withFallback({ ByteArray(length).takeIf { hardwareFill(it) } }) {
            val raw = try {
                Base64.getDecoder().decode(post("GenerateBytes", length).trim())
            } catch (e: IllegalArgumentException) {
                throw RdRandException("invalid base64 from the API", e)
            }
            if (raw.size != length) throw RdRandException("expected $length bytes from the API, got ${raw.size}")
            raw
        }
    }

    /** A random integer from 0 to [Int.MAX_VALUE] (2^31 - 1), the API's range. */
    fun generateUInt(): Int = withFallback({
        val b = ByteArray(4)
        if (!hardwareFill(b)) {
            null
        } else {
            ((b[0].toInt() and 0xff) or ((b[1].toInt() and 0xff) shl 8) or
                ((b[2].toInt() and 0xff) shl 16) or ((b[3].toInt() and 0x7f) shl 24))
        }
    }) {
        val s = post("GenerateUInt", null).trim()
        s.toIntOrNull()?.takeIf { it >= 0 } ?: throw RdRandException("not an integer from the API: ${s.take(40)}")
    }

    private fun <T : Any> withFallback(hardware: () -> T?, remote: () -> T): T {
        if (mode != Mode.SERVICE) {
            hardware()?.let { return it }
            if (mode == Mode.HARDWARE) {
                throw RdRandException(
                    if (hardwareSource() != null) "the hardware random number generator failed to deliver"
                    else "this CPU has no working hardware random number generator",
                )
            }
        }
        return remote()
    }

    /** Uniform over the 63-character alphabet: bytes 252..255 are rejected. */
    private fun keyFromHardware(length: Int): String? {
        val out = StringBuilder(length)
        while (out.length < length) {
            val want = length - out.length
            val buf = ByteArray(minOf(want + want / 32 + 8, 65536))
            if (!hardwareFill(buf)) return null
            for (b in buf) {
                val v = b.toInt() and 0xff
                if (v < 252 && out.length < length) out.append(KEY_ALPHABET[v % 63])
            }
        }
        return out.toString()
    }

    private fun post(action: String, length: Int?): String {
        val builder = HttpRequest.newBuilder(URI.create("$baseUrl/$action"))
            .timeout(timeout)
            .header("User-Agent", "rdrand-client-kotlin/$VERSION")
        val request = if (length != null) {
            builder.header("Content-Type", "application/x-www-form-urlencoded")
                .POST(HttpRequest.BodyPublishers.ofString("length=$length"))
        } else {
            builder.POST(HttpRequest.BodyPublishers.noBody())
        }.build()
        val response = try {
            http.send(request, HttpResponse.BodyHandlers.ofString())
        } catch (e: IOException) {
            throw RdRandException("rdrand.com API: ${e.message}", e)
        } catch (e: InterruptedException) {
            Thread.currentThread().interrupt()
            throw RdRandException("rdrand.com API: interrupted", e)
        }
        if (response.statusCode() != 200) throw RdRandException("rdrand.com API: HTTP ${response.statusCode()}")
        return response.body()
    }

    private fun requireLength(length: Int) {
        if (length < 0) throw IllegalArgumentException("length must not be negative")
    }
}
