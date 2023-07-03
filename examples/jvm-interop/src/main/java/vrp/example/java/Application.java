package vrp.example.java;

import com.sun.jna.Callback;
import com.sun.jna.Library;
import com.sun.jna.Native;

import java.io.IOException;
import java.nio.file.Files;
import java.nio.file.Paths;

/** Encapsulate Vehicle Routing Problem solver behavior.  */
interface Solver extends Library {
    /** Gets list of routing matrix locations. **/
    void get_routing_locations(String problem, OnSuccess onSuccess, OnError onError);
    /** Converts problem to pragmatic format. **/
    void convert_to_pragmatic(String format, String[] inputs, int inputsLen, OnSuccess onSuccess, OnError onError);
    /** Solves pragmatic problem. **/
    void solve_pragmatic(String problem, String[] matrices,
                         int matricesSize,
                         String config,
                         boolean geojson,
                         OnSuccess onSuccess, OnError onError);
}

interface OnSuccess extends Callback {
    void result(String json);
}

interface OnError extends Callback {
    void result(String error);
}

class Application {
    public static void main(String[] args) throws IOException {
        if (args.length < 1) {
            throw new IllegalStateException("Specify problem and, optionally, routing matrices paths");
        }

        String problem = new String(Files.readAllBytes(Paths.get(args[0])));
        String[] matrices = new String[args.length - 1];
        for (int i = 1; i < args.length; i++) {
            matrices[i - 1] = new String(Files.readAllBytes(Paths.get(args[i])));
        }

        Solver solver = Native.load("vrp_cli", Solver.class);

        solver.get_routing_locations(problem,
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

        solver.solve_pragmatic(problem, matrices, matrices.length, "{}", false
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
