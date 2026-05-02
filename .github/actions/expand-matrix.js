const TOOL_OS_LIST = {};

const TOOL_VERSION_ARG = {};

const TOOL_REQS = {
	npm: ["node"],
	pnpm: ["node"],
	poetry: ["python"],
	yarn: ["node"],
};

const tools = [];

// TODO ruby?
["bun", "deno", "go", "moon", "node", "npm", "pnpm", "poetry", "python", "uv", "yarn"].forEach((tool) => {
	const osList = TOOL_OS_LIST[tool] || ["macos-latest", "ubuntu-latest", "windows-latest"];

	osList.forEach((os) => {
		tools.push({
			tool,
			os,
			requires: TOOL_REQS[tool] || [],
			versionArg: TOOL_VERSION_ARG[tool] || "--version"
		});
	});
});

console.log(`matrix=${JSON.stringify(tools)}`);
