import importlib.util
from pathlib import Path
import tempfile
import textwrap
import unittest


SCRIPT_PATH = Path(__file__).parents[1] / "generate_community_registry.py"
SPEC = importlib.util.spec_from_file_location("generate_community_registry", SCRIPT_PATH)
assert SPEC and SPEC.loader
GENERATOR = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(GENERATOR)


class GenerateCommunityRegistryTest(unittest.TestCase):
    def write_manifest(self, root: Path, name: str, contents: str) -> None:
        tools_dir = root / "tools"
        tools_dir.mkdir(exist_ok=True)
        (tools_dir / f"{name}.toml").write_text(textwrap.dedent(contents))

    def test_maps_metadata_and_plugin_configuration(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.write_manifest(
                root,
                "example",
                """
                name = "Example"

                [plugin]
                description = "An example tool."
                homepage-url = "https://example.com/"
                repository-url = "https://github.com/example/example"

                [detect]
                version-files = [".example-version", "example.json"]

                [install]
                download-url = "https://example.com/{version}"

                [install.exes.helper]
                exe-path = "bin/helper"

                [packages]
                globals-lookup-dirs = ["$EXAMPLE_HOME/bin", "~/.example/bin"]
                """,
            )

            [plugin] = GENERATOR.generate_registry(root)["plugins"]

            self.assertEqual(plugin["id"], "example")
            self.assertEqual(plugin["name"], "Example")
            self.assertEqual(plugin["description"], "An example tool.")
            self.assertEqual(plugin["homepageUrl"], "https://example.com/")
            self.assertEqual(
                plugin["repositoryUrl"], "https://github.com/example/example"
            )
            self.assertEqual(plugin["bins"], ["example", "helper"])
            self.assertEqual(
                plugin["detectionSources"],
                [{"file": ".example-version"}, {"file": "example.json"}],
            )
            self.assertEqual(
                plugin["globalsDirs"], ["$EXAMPLE_HOME/bin", "~/.example/bin"]
            )

    def test_uses_explicit_primary_bin_and_sorts_plugins(self) -> None:
        with tempfile.TemporaryDirectory() as temp_dir:
            root = Path(temp_dir)
            self.write_manifest(root, "zulu", 'name = "Zulu"\n')
            self.write_manifest(
                root,
                "alpha",
                """
                name = "Alpha"

                [install.exes.alpha-cli]
                primary = true

                [install.exes.alpha-helper]
                """,
            )

            plugins = GENERATOR.generate_registry(root)["plugins"]

            self.assertEqual([plugin["id"] for plugin in plugins], ["alpha", "zulu"])
            self.assertEqual(plugins[0]["bins"], ["alpha-cli", "alpha-helper"])
            self.assertEqual(plugins[1]["bins"], ["zulu"])
            self.assertEqual(plugins[1]["description"], "")
            self.assertEqual(
                plugins[1]["repositoryUrl"], GENERATOR.COMMUNITY_REPOSITORY
            )


if __name__ == "__main__":
    unittest.main()
