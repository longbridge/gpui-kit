"""A stale or missing published recipe must fail before consumers copy it."""
import json
from pathlib import Path
import runpy
import tempfile
import unittest

fragments = runpy.run_path(str(Path(__file__).resolve().parents[1] / "check-ai-recipes"))["fragments"]


class PublishedRecipes(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory()
        self.addCleanup(self.directory.cleanup)
        self.root = Path(self.directory.name)
        (self.root / "examples/ai_recipes").mkdir(parents=True)
        (self.root / "source.rs").write_text("fn main() {}\n")
        (self.root / "examples/ai_recipes/recipes.json").write_text(json.dumps([
            {"id": "startup", "source": "source.rs", "documents": ["guide.md"]}
        ]))
        (self.root / "guide.md").write_text("Before\n<!-- recipe:startup:start -->\n```rust\nfn main() {}\n```\n<!-- recipe:startup:end -->\nAfter\n")

    def test_changed_source_rejects_stale_published_code(self):
        self.assertEqual(fragments(self.root), 1)
        (self.root / "source.rs").write_text("fn main() { run(); }\n")
        with self.assertRaisesRegex(ValueError, "stale startup"):
            fragments(self.root)
        fragments(self.root, sync=True)
        self.assertEqual(fragments(self.root), 1)
        text = (self.root / "guide.md").read_text()
        self.assertTrue(text.startswith("Before\n"))
        self.assertTrue(text.endswith("After\n"))
        self.assertIn("fn main() { run(); }", text)

    def test_removed_or_duplicate_markers_cannot_silently_drop_coverage(self):
        for text in ["No recipe", "<!-- recipe:startup:start --><!-- recipe:startup:start --><!-- recipe:startup:end -->"]:
            (self.root / "guide.md").write_text(text)
            with self.assertRaisesRegex(ValueError, "missing or duplicate"):
                fragments(self.root)

    def test_empty_inventory_cannot_report_success(self):
        (self.root / "examples/ai_recipes/recipes.json").write_text("[]")
        with self.assertRaisesRegex(ValueError, "no published fragments"):
            fragments(self.root)


if __name__ == "__main__":
    unittest.main()
