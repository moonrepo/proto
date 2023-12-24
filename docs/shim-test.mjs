console.log("start");

process.on("SIGINT", () => {
  console.log("interrupted");
  process.exit(2);
});

process.on("SIGTERM", () => {
  console.log("terminated");
  process.exit(3);
});

process.on("SIGHUP", () => {
  console.log("hangup");
  process.exit(4);
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
}, 1000 * 60 * 1);

console.log("running");
