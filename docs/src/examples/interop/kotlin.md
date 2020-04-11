# Kotlin

This is example how to call solver methods from **kotlin**. You need to make sure that `vrp-cli` library is available
in runtime, e.g. by copying corresponding binary (`libvrp_cli.so` on Linux) to `resources` directory. To build it, use
the following command;

    cargo build --release

```kotlin
{{#include ../../../../examples/jvm-interop/src/main/kotlin/vrp/example/kotlin/Application.kt}}
```

You can check the project repository for complete example.