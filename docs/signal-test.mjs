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

await new Promise(() => {
    setTimeout(() => {
        process.exit(0);
    }, 30000);
})