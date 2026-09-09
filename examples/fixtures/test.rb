require 'json'
require 'date'

# Module for logging functionality
module Logging
  LOG_LEVELS = %i[debug info warn error].freeze
end

# HelloWorld class provides greeting functionality with configuration options
# @author Example Author
# @version 1.0.0
# Features:
# - Configurable greetings with multiple names
# - Instance tracking
# - Report generation
# - Logging capabilities
class HelloWorld < Object
  include Logging

  @@instances = 0
  VERSION = '1.0.0'

  attr_accessor :name
  attr_reader :created_at

  def initialize(name: 'World', options: {})
    @name = name
    @created_at = Time.now
    @options = options
    @@instances += 1
    yield self if block_given?
  end

  def self.instance_count(format: :short)
    case format
    when :short then @@instances.to_s
    when :long then "Total instances: #{@@instances}"
    end
  end

  def greet(*names)
    names.each { |n| puts "Hello, #{n}!" }
  rescue => e
    puts "Error: #{e.message}"
  end

  def configure(timeout: 5000, retries: 3)
    @options.merge!(timeout: timeout, retries: retries)
  end

  def configured?
    !@options.empty?
  end

  def process_names(names)
    names.map(&:upcase).select(&:present?)
  end

  private

  def generate_report
    <<~REPORT
      HelloWorld Report
      ================
      Name: #{@name}
      Created: #{@created_at}
      Options: #{@options.to_json}
    REPORT
  end
end

# Create new greeter instance with configuration block
greeter = HelloWorld.new(name: 'Ruby') { |g| g.configure(timeout: 1000) }

# Process array and handle errors
numbers = [1, 2, 3, 4, 5]
doubled = numbers.map { |n| n * 2 }

# Email validation
EMAIL_REGEX = /\A[\w+\-.]+@[a-z\d\-]+(\.[a-z\d\-]+)*\.[a-z]+\z/i
validator = ->(email) { email.match?(EMAIL_REGEX) }

begin
  greeter.greet('Alice', 'Bob')
rescue StandardError => e
  puts "Error occurred: #{e.message}"
ensure
  puts "Execution completed at #{Time.now}"
end
