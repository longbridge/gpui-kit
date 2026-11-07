package hello

import kotlin.math.roundToInt

const val VERSION = "1.0.0"

data class Config(
    val timeout: Int = 5000,
    val retries: Int = 3,
    val debug: Boolean = false,
)

enum class LogLevel(val label: String) {
    INFO("INFO"), WARN("WARN"), ERROR("ERROR");
    companion object {
        fun fromString(s: String): LogLevel? = entries.find { it.label == s }
    }
}

sealed class Result<out T> {
    data class Success<T>(val data: T) : Result<T>()
    data class Error(val message: String) : Result<Nothing>()
}

interface Greetable {
    fun greet(vararg names: String): List<String>
}

class HelloWorld(private val name: String = "World") : Greetable {
    private var config = Config()
    private val createdAt = System.currentTimeMillis()

    override fun greet(vararg names: String): List<String> {
        return names.map { "Hello, $it!" }.also { lines ->
            if (config.debug) lines.forEach { println("  [debug] $it") }
        }
    }

    fun configure(block: Config.() -> Config) {
        config = config.block()
    }

    fun processNames(names: List<String>?): List<String> =
        names?.filter { it.isNotBlank() }
            ?.map { it.uppercase() }
            ?.sorted()
            ?: emptyList()

    fun generateReport(): String {
        val elapsed = ((System.currentTimeMillis() - createdAt) / 1000.0).roundToInt()
        return """
            |HelloWorld Report
            |=================
            |Name: $name
            |Elapsed: ${elapsed}s
            |Config: timeout=${config.timeout}, retries=${config.retries}
        """.trimMargin()
    }

    override fun toString(): String = "HelloWorld(name=$name)"
}

fun <T> safely(block: () -> T): Result<T> = try {
    Result.Success(block())
} catch (e: Exception) {
    Result.Error(e.message ?: "unknown error")
}

fun describe(obj: Any?): String = when (obj) {
    null -> "null"
    is String -> "String(${obj.length}): \"${obj.take(20)}\""
    is Result.Success<*> -> "ok: ${obj.data}"
    is Result.Error -> "err: ${obj.message}"
    else -> obj::class.simpleName ?: "unknown"
}

fun main() {
    val greeter = HelloWorld("Kotlin")

    greeter.configure { copy(debug = true, retries = 5) }
    greeter.greet("Alice", "Bob", "Charlie")

    val processed = greeter.processNames(listOf("alice", "", "bob"))
    println("Processed: $processed")

    val result = safely { greeter.generateReport() }
    when (result) {
        is Result.Success -> println(result.data)
        is Result.Error -> println("Failed: ${result.message}")
    }

    // Null safety & describe
    val items: List<Any?> = listOf("hello", 42, null, result)
    items.forEach { println("  ${describe(it)}") }

    println("Instances: $greeter (v$VERSION)")
}
