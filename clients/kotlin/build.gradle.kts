import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    kotlin("jvm") version "2.1.20"
}

group = "com.jebtek"
version = "2.0.0"

repositories {
    mavenCentral()
}

kotlin {
    jvmToolchain(17)
    // java.net.http needs Java 11; keep the bytecode usable there.
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_11)
    }
}

java {
    targetCompatibility = JavaVersion.VERSION_11
    sourceCompatibility = JavaVersion.VERSION_11
}

tasks.jar {
    archiveBaseName.set("rdrand-kotlin")
    manifest {
        attributes(
            "Implementation-Title" to "rdrand-kotlin",
            "Implementation-Version" to project.version,
            "Implementation-Vendor" to "JebteK, LLC",
            "Automatic-Module-Name" to "com.jebtek.rdrand",
        )
    }
}

// The tests are a plain main() so they also run from a self-contained jar on
// machines with only a JRE: `java -jar rdrand-kotlin-selftest.jar`.
val selfTest by tasks.registering(JavaExec::class) {
    group = "verification"
    classpath = sourceSets.test.get().runtimeClasspath
    mainClass.set("com.jebtek.rdrand.SelfTestKt")
}
tasks.test { dependsOn(selfTest) }

val selfTestJar by tasks.registering(Jar::class) {
    archiveBaseName.set("rdrand-kotlin-selftest")
    manifest { attributes("Main-Class" to "com.jebtek.rdrand.SelfTestKt") }
    duplicatesStrategy = DuplicatesStrategy.EXCLUDE
    from(sourceSets.main.get().output, sourceSets.test.get().output)
    from({ sourceSets.test.get().runtimeClasspath.filter { it.name.endsWith(".jar") }.map { zipTree(it) } })
    exclude("META-INF/*.SF", "META-INF/*.DSA", "META-INF/*.RSA")
}
