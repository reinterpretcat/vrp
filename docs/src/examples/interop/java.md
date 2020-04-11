# Java

This is example how to call solver methods from **java**. You need to make sure that `vrp-cli` library is available
in runtime, e.g. by copying corresponding binary (`libvrp_cli.so` on Linux) to `resources` directory. To build it, use
the following command:

    cargo build --release

```java
{{#include ../../../../examples/jvm-interop/src/main/java/vrp/example/java/Application.java}}
```

You can check the project repository for complete example.