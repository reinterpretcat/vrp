package vrp.example.java;

import com.sun.jna.Callback;
import com.sun.jna.Library;
import com.sun.jna.Native;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

interface OnSuccess extends Callback {
    void result(String json);
}

interface OnError extends Callback {
    void result(String error);
}

interface Solver extends Library {
    void solve(String problem, String[] matrices, int matricesSize, OnSuccess onSuccess, OnError onError);
}

class Application {
    public static void main(String[] args) throws IOException {
        if (args.length < 2) {
            throw new IllegalStateException("Specify problem and routing matrices paths");
        }

        String problem = new String(Files.readAllBytes(Paths.get(args[0])));
        String[] matrices = new String[args.length - 1];
        for (int i = 1; i < args.length; i++) {
            matrices[i] = new String(Files.readAllBytes(Paths.get(args[i])));
        }

        Solver solver = Native.load("vrp_pragmatic", Solver.class);

        solver.solve(problem, matrices, matrices.length,
                new OnSuccess() {
                    @Override
                    public void result(String json) {
                        System.out.println(json);
                    }
                }, new OnError() {
                    @Override
                    public void result(String error) {
                        System.out.println(error);
                    }
                });
    }
}
