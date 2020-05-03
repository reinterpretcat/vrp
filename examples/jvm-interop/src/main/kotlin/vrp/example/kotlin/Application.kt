package vrp.example.kotlin

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import java.nio.file.Files
import java.nio.file.Paths

/** Encapsulate Vehicle Routing Problem solver behavior.  */
private interface Solver : Library {
    /** Gets list of routing matrix locations **/
    fun get_routing_locations(problem: String, onSuccess: OnSuccess, onError: OnError)
    /** Converts problem to pragmatic format. **/
    fun convert_to_pragmatic(format: String, inputs: Array<String>, inputsLen: Int, onSuccess: OnSuccess, onError: OnError)
    /** Solves pragmatic problem. maxTime is in seconds. **/
    fun solve_pragmatic(problem: String,
                        matrices: Array<String>,
                        matricesLen: Int,
                        config: String,
                        onSuccess: OnSuccess, onError: OnError)
}

private interface OnSuccess : Callback {
    fun result(json: String)
}

private interface OnError : Callback {
    fun result(error: String)
}

fun main(args: Array<String>) {
    if (args.count() < 1) {
        throw IllegalStateException("Specify problem and, optionally, routing matrices paths")
    }

    val problem = String(Files.readAllBytes(Paths.get(args[0])))
    val matrices = args.drop(1).map { String(Files.readAllBytes(Paths.get(it))) }.toTypedArray()

    val solver = Native.load("vrp_cli", Solver::class.java)

    solver.get_routing_locations(problem,
            onSuccess = object : OnSuccess {
                override fun result(json: String) {
                    println("locations: $json")
                }
            },
            onError = object : OnError {
                override fun result(error: String) {
                    println(error)
                }
            }
    )

    solver.solve_pragmatic(problem, matrices, matrices.size, "{}",
            onSuccess = object : OnSuccess {
                override fun result(json: String) {
                    println("solution: $json")
                }
            },
            onError = object : OnError {
                override fun result(error: String) {
                    println(error)
                }
            }
    )
}
