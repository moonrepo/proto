console.log("start");

// Test CTRL+C handling
process.on("SIGINT", () => {
  console.log("killed");
  process.exit(1);
});

// Test piping input
async function getStdinBuffer() {
  if (process.stdin.isTTY) {
    return Buffer.alloc(0);
  }

  const result = [];
  let length = 0;

  for await (const chunk of process.stdin) {
    result.push(chunk);
    length += chunk.length;
  }

  return Buffer.concat(result, length);
}

getStdinBuffer().then((buffer) => {
  let data = buffer.toString("utf8");

  if (data) {
    console.log("piped data =", data.trim());
  }
});

// Start a timer so we can ensure "stop" is never logged
setTimeout(() => {
  console.log("stop");
}, 5000);

console.log("running");
