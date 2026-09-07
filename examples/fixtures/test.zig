const std = @import("std");
const json = std.json;
const time = std.time;
const HashMap = std.HashMap;

pub const VERSION = "1.0.0";

pub const HelloError = error{
    InvalidName,
    Timeout,
};

pub const HelloWorld = struct {
    name: []const u8,
    options: HashMap([]const u8, json.Value),
    created_at: i64,

    pub fn init(allocator: *std.mem.Allocator, name: []const u8) !HelloWorld {
        return HelloWorld{
            .name = name,
            .options = HashMap([]const u8, json.Value).init(allocator),
            .created_at = time.timestamp(),
        };
    }

    pub fn deinit(self: *HelloWorld) void {
        self.options.deinit();
    }

    pub fn greet(self: *const HelloWorld, names: []const []const u8) !void {
        for (names) |name| {
            time.sleep(100 * time.millisecond);
            std.debug.print("Hello, {s}!\n", .{name});
        }
    }

    pub fn configure(self: *HelloWorld, options: HashMap([]const u8, json.Value)) void {
        var it = options.iterator();
        while (it.next()) |entry| {
            self.options.put(entry.key, entry.value) catch {};
        }
    }

    pub fn generateReport(self: *const HelloWorld) ![]const u8 {
        var report = std.ArrayList(u8).init(std.heap.page_allocator);
        defer report.deinit();

        try report.writer().print(
            \\HelloWorld Report
            \\================
            \\Name: {s}
            \\Created: {}
            \\Options: {}
            \\
        , .{
            self.name,
            self.created_at,
            self.options,
        });

        return report.toOwnedSlice();
    }
};

pub fn main() !void {
    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = &gpa.allocator;

    var greeter = try HelloWorld.init(allocator, "Zig");
    defer greeter.deinit();

    var config = HashMap([]const u8, json.Value).init(allocator);
    try config.put("timeout", json.Value{ .Integer = 5000 });
    try config.put("retries", json.Value{ .Integer = 3 });
    
    greeter.configure(config);

    const names = [_][]const u8{ "Alice", "Bob" };
    try greeter.greet(&names);

    const report = try greeter.generateReport();
    std.debug.print("{s}\n", .{report});
}
