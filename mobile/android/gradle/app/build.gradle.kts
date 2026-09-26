plugins {
    id("com.android.application")
}

android {
    namespace = "dev.apichat.mobile"
    compileSdk = 37

    defaultConfig {
        applicationId = "dev.apichat.mobile"
        minSdk = 26
        targetSdk = 37
        versionCode = 1
        versionName = "0.1.0"
        ndk { abiFilters += listOf("arm64-v8a", "x86_64") }
        manifestPlaceholders["nativeLibraryName"] = "api_chat_mobile"
    }

    buildTypes {
        release { isMinifyEnabled = false }
        debug { isDebuggable = true; isJniDebuggable = true }
    }

    sourceSets {
        getByName("main") {
            jniLibs.directories.add("src/main/jniLibs")
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}
