const fs = require('fs');

let config = [];

for (let i = 2; i < process.argv.length; i++) {
	const tool = process.argv[i];
	config.push(`${tool} = "latest"`);
}

fs.writeFileSync('.prototools', config.join("\n"));
