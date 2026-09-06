const fs = require("fs");

const getName = function () {
  return "John Doe";
};

/**
 * A class representing a HelloWorld greeter with various utility methods
 * @class HelloWorld
 * @param {string} name - The name to use for greetings
 */
class HelloWorld {
  // Version number of the HelloWorld class
  static VERSION = "1.0.0";
  // Counter to track number of class instances
  static #instanceCount = 0;

  // Private instance fields
  #name;
  #options;
  #createdAt;

  /**
   * Creates a new HelloWorld instance
   * @param {string} name - The name to use for greetings
   * @param {Object} options - Configuration options
   */
  constructor(name = "World", options = {}) {
    this.#name = name;
    this.#options = options;
    this.#createdAt = new Date();
    HelloWorld.#instanceCount++;
  }

  static getInstanceCount() {
    return HelloWorld.#instanceCount;
  }

  get name() {
    return this.#name;
  }

  set name(value) {
    this.#name = value;
  }

  async greet(...names) {
    try {
      for (const name of names) {
        await new Promise((resolve) => setTimeout(resolve, 100));
        console.log(`Hello, ${name}!`);
      }
    } catch (error) {
      console.error(`Error: ${error.message}`);
    }
  }

  configure(options = {}) {
    Object.assign(this.#options, options);
  }

  *generateSequence(start = 0, end = 10) {
    for (let i = start; i <= end; i++) yield i;
  }

  processNames(names) {
    return names
      .filter((name) => name.length > 0)
      .map((name) => name.toUpperCase())
      .sort();
  }
}

const greeter = new HelloWorld("JavaScript");

(async () => {
  const uniqueNames = new Set(["Alice", "Bob"]);
  await greeter.greet(...uniqueNames);

  for (const num of greeter.generateSequence(0, 5)) {
    console.log(num);
  }
})();
