from __future__ import annotations
from typing import Optional, List, Dict, Any
from dataclasses import dataclass
from datetime import datetime
import json
import asyncio

# Data class for configuration settings
@dataclass
class Config:
    timeout: int = 5000  # Default timeout in milliseconds
    retries: int = 3     # Number of retry attempts
    debug: bool = False  # Debug mode flag

"""
HelloWorld class provides greeting functionality with configuration options.

Features:
- Async greetings with customizable names
- Configuration management
- Instance tracking
- Report generation

Example:
    greeter = HelloWorld("Python")
    await greeter.greet("Alice", "Bob")
"""
class HelloWorld:
    VERSION: str = "1.0.0"
    _instance_count: int = 0
    
    def __init__(self, name: str = "World", options: Optional[Dict[str, Any]] = None):
        self._name = name
        self._options = options or {}
        self._created_at = datetime.now()
        self._config = Config()
        HelloWorld._instance_count += 1
    
    @property
    def name(self) -> str:
        return self._name
    
    @name.setter
    def name(self, value: str) -> None:
        if not value:
            raise ValueError("Name cannot be empty")
        self._name = value
    
    @classmethod
    def get_instance_count(cls) -> int:
        return cls._instance_count
    
    async def greet(self, *names: str) -> None:
        try:
            for name in names:
                await asyncio.sleep(0.1)
                print(f"Hello, {name}!")
        except Exception as e:
            print(f"Error: {str(e)}")
    
    def process_names(self, names: List[str] = None) -> List[str]:
        if names is None:
            names = []
        return sorted([name.upper() for name in names if name])
    
    def _generate_report(self) -> str:
        return f"""
        HelloWorld Report
        ================
        Name: {self._name}
        Created: {self._created_at.isoformat()}
        Options: {json.dumps(self._options, indent=2)}
        """
    
    def __str__(self) -> str:
        return f"HelloWorld(name={self._name})"

async def main():
    greeter = HelloWorld("Python")
    await greeter.greet("Alice", "Bob", "Charlie")
    print(greeter._generate_report())

if __name__ == "__main__":
    asyncio.run(main())
