--[[
    HelloWorld module demonstrating Lua syntax highlighting
    Version: 1.0.0
--]]

local VERSION = "1.0.0"

-- Module configuration with default values
local Config = {
    timeout = 5000,
    retries = 3,
    debug = false
}

-- Enum-like table for log levels
local LogLevel = {
    INFO = "INFO",
    WARN = "WARN",
    ERROR = "ERROR"
}

-- Create a class using metatables
local HelloWorld = {}
HelloWorld.__index = HelloWorld

function HelloWorld.new(name)
    local self = setmetatable({}, HelloWorld)
    self.name = name or "World"
    self.config = {timeout = 5000, retries = 3, debug = false}
    self.createdAt = os.time()
    self._greetCount = 0  -- private-like field
    return self
end

-- Method using colon syntax (implicit self)
function HelloWorld:greet(...)
    local names = {...}
    local results = {}

    for i, name in ipairs(names) do
        local greeting = string.format("Hello, %s!", name)
        table.insert(results, greeting)
        self._greetCount = self._greetCount + 1

        if self.config.debug then
            print(string.format("  [debug] %s", greeting))
        end
    end

    return results
end

function HelloWorld:configure(newConfig)
    for k, v in pairs(newConfig) do
        self.config[k] = v
    end
end

function HelloWorld:processNames(names)
    local processed = {}

    for _, name in ipairs(names or {}) do
        if name ~= "" and name:match("%S") then
            table.insert(processed, name:upper())
        end
    end

    table.sort(processed)
    return processed
end

function HelloWorld:generateReport()
    local elapsed = os.time() - self.createdAt
    local lines = {
        "HelloWorld Report",
        "=================",
        string.format("Name: %s", self.name),
        string.format("Elapsed: %ds", elapsed),
        string.format("Greetings: %d", self._greetCount),
        string.format("Config: timeout=%d, retries=%d",
            self.config.timeout, self.config.retries)
    }
    return table.concat(lines, "\n")
end

-- Metatable for Result type (Success/Error)
local function Success(data)
    return {success = true, data = data}
end

local function Error(message)
    return {success = false, message = message}
end

-- Protected call wrapper
local function safely(fn)
    local status, result = pcall(fn)
    return status and Success(result) or Error(tostring(result))
end

-- Pattern matching-like function
local function describe(obj)
    if obj == nil then return "nil"
    elseif type(obj) == "string" then
        return string.format('String(%d): "%s"', #obj, obj:sub(1, 20))
    elseif type(obj) == "table" and obj.success ~= nil then
        return obj.success and ("ok: " .. tostring(obj.data))
                           or ("err: " .. obj.message)
    else
        return type(obj)
    end
end

-- Main execution
local greeter = HelloWorld.new("Lua")

greeter:configure({debug = true, retries = 5})
greeter:greet("Alice", "Bob", "Charlie")

local processed = greeter:processNames({"alice", "", "bob", "  charlie  "})
print("Processed: " .. table.concat(processed, ", "))

local result = safely(function() return greeter:generateReport() end)
if result.success then
    print(result.data)
else
    print("Failed: " .. result.message)
end

print(string.format("\nVersion: %s", VERSION))
