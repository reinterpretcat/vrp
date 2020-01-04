package vrp.example.kotlin

import com.sun.jna.Callback
import com.sun.jna.Library
import com.sun.jna.Native
import java.nio.file.Files
import java.nio.file.Paths

private interface OnSuccess : Callback {
    fun result(json: String)
}

private interface OnError : Callback {
    fun result(error: String)
}

private interface Solver : Library {
    fun solve(problem: String, matrices: Array<String>, matrices_size: Int, onSuccess: OnSuccess, onError: OnError)
}

fun main(args: Array<String>) {
    if (args.count() < 2) {
        throw IllegalStateException("Specify problem and routing matrices paths")
    }

    val problem = String(Files.readAllBytes(Paths.get(args[0])))
    val matrices = args.drop(1).map { String(Files.readAllBytes(Paths.get(it))) }.toTypedArray()

    val solver = Native.load("vrp_pragmatic", Solver::class.java)

    solver.solve(problem, matrices, matrices.size,
            onSuccess = object : OnSuccess {
                override fun result(json: String) {
                    println(json)
                }
            },
            onError = object : OnError {
                override fun result(error: String) {
                    println(error)
                }
            }
    )
}
