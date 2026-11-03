package com.jebtek.rdrand

import kotlin.system.exitProcess

// `gradle test`, or `java -jar rdrand-kotlin-selftest.jar` on any JRE 11+.
// Set RDRAND_TEST_URL (e.g. https://rdrand.com/API/) to also exercise the API.

private var failures = 0

private fun check(name: String, ok: Boolean, detail: String = "") {
    println("  [${if (ok) "PASS" else "FAIL"}] $name${if (detail.isEmpty()) "" else ": $detail"}")
    if (!ok) failures++
}

private fun isKey(s: String, n: Int) = s.length == n && s.all { it in RdRand.KEY_ALPHABET }

fun main() {
    val os = "${System.getProperty("os.name")} ${System.getProperty("os.arch")}, Java ${System.getProperty("java.version")}"
    val hw = RdRand.hardwareSource()
    println("hardware generator: ${hw?.id ?: "none"} ($os)")
    check("auto source", RdRand().source == (hw ?: Source.SERVICE), RdRand().source.id)

    val hwRng = RdRand(Mode.HARDWARE)
    if (hw == null) {
        check("hardware mode refuses", runCatching { hwRng.generateKey(4) }.exceptionOrNull() is RdRandException)
    } else {
        check("hardware key", isKey(hwRng.generateKey(5000), 5000))
        check("hardware empty key", hwRng.generateKey(0).isEmpty())
        val bytes = hwRng.generateBytes(1 shl 20)
        var zeroWords = 0
        for (i in bytes.indices step 4) {
            if (bytes[i].toInt() or bytes[i + 1].toInt() or bytes[i + 2].toInt() or bytes[i + 3].toInt() == 0) zeroWords++
        }
        check("hardware 1 MiB bytes, no zero words", bytes.size == 1 shl 20 && zeroWords == 0, "$zeroWords zero words")
        val n = hwRng.generateUInt()
        check("hardware uint in range", n >= 0, "$n")
        val key = hwRng.generateKey(252_000)
        val counts = key.groupingBy { it }.eachCount()
        val expected = key.length / 63.0
        val chi2 = counts.values.sumOf { (it - expected) * (it - expected) / expected }
        check("hardware key uniform", counts.size == 63 && chi2 < 120, "chi2=%.1f".format(chi2))
    }

    val url = System.getenv("RDRAND_TEST_URL")
    if (url == null) {
        println("  [SKIP] service mode: RDRAND_TEST_URL not set")
    } else {
        val svc = RdRand(Mode.SERVICE, url)
        check("service source", svc.source == Source.SERVICE)
        check("service key", isKey(svc.generateKey(100), 100))
        check("service empty key", svc.generateKey(0).isEmpty())
        check("service bytes", svc.generateBytes(1000).size == 1000)
        check("service uint", svc.generateUInt() >= 0)
        check(
            "service error surfaces",
            runCatching { RdRand(Mode.SERVICE, url + "Nope/").generateUInt() }.exceptionOrNull() is RdRandException,
        )
    }

    val auto = RdRand(baseUrl = url ?: RdRand.DEFAULT_BASE_URL)
    check("auto key", isKey(auto.generateKey(32), 32))
    check("negative length rejected", runCatching { auto.generateBytes(-1) }.exceptionOrNull() is IllegalArgumentException)

    println(if (failures == 0) "all checks passed" else "$failures check(s) FAILED")
    exitProcess(if (failures == 0) 0 else 1)
}
