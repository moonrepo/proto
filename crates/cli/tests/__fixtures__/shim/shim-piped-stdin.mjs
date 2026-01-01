console.log("start");

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

setTimeout(() => {
  console.log("stop");
}, 2500);

console.log("running");
