<?php

// This file is only used for syntax highlighting fixtures.
// Intentionally small and dependency-free.

declare(strict_types=1);

final class HelloWorld
{
    public const VERSION = '1.0.0';

    public function __construct(private string $name = 'World')
    {
    }

    public function greet(string ...$names): array
    {
        return array_map(fn(string $n): string => "Hello, {$n}!", $names);
    }

    public function report(): string
    {
        return "HelloWorld({$this->name})";
    }
}

function is_valid_email(string $email): bool
{
    return preg_match('/^[\w+\-.]+@[\w\-]+\.[a-z]{2,}$/i', $email) === 1;
}

$name = $_GET['name'] ?? 'PHP';
$email = $_POST['email'] ?? 'user@example.com';

$greeter = new HelloWorld((string) $name);
$lines = $greeter->greet('Alice', 'Bob', 'Charlie');

?>
<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8" />
  <title><?php echo htmlspecialchars($greeter->report(), ENT_QUOTES, 'UTF-8'); ?></title>
</head>
<body>
  <h1><?php echo htmlspecialchars($greeter->report(), ENT_QUOTES, 'UTF-8'); ?></h1>

  <ul>
    <?php foreach ($lines as $line): ?>
      <li><?php echo htmlspecialchars($line, ENT_QUOTES, 'UTF-8'); ?></li>
    <?php endforeach; ?>
  </ul>

  <form method="post">
    <label>
      Email:
      <input name="email" value="<?php echo htmlspecialchars((string) $email, ENT_QUOTES, 'UTF-8'); ?>" />
    </label>
    <button type="submit">Validate</button>
  </form>

  <?php if ($email !== ''): ?>
    <p>Status: <strong><?php echo is_valid_email((string) $email) ? 'valid' : 'invalid'; ?></strong></p>
  <?php endif; ?>
</body>
</html>
