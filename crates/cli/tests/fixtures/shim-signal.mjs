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

setTimeout(() => {
  console.log("stop");
}, 5000);

console.log("running");
