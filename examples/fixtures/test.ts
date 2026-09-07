import fs, { readFile } from "fs";

const VERSION = "1.0.0";

class HelloWorld {
  private name: string;
  private createdAt: Date;
  private options: Record<string, any>;

  constructor(name: string) {
    this.name = name;
    this.createdAt = new Date();
    this.options = {};
  }

  greet(names: string[]): void {
    for (const name of names) {
      console.log(`Hello, ${name}!`);
    }
  }

  configure(cfg: { timeout: number; retries: number; debug: boolean }): void {
    this.options["timeout"] = cfg.timeout;
    this.options["retries"] = cfg.retries;
    this.options["debug"] = cfg.debug;
  }

  generateReport(): string {
    return `
        HelloWorld Report
        =================
        Name: ${this.name}
        Created: ${this.createdAt.toISOString()}
        Options: ${JSON.stringify(this.options, null, 2)}
    `;
  }
}

const timeout = 5000;

function main() {
  const greeter = new HelloWorld("TypeScript");
  greeter.configure({
    timeout: timeout,
    retries: 3,
    debug: true,
  });

  greeter.greet(["Alice", "Bob"]);
  console.log(greeter.generateReport());
}
