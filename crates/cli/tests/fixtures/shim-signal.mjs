console.log("start");

process.on("SIGINT", () => {
  console.log("killed");
  process.exit(1);
});

setTimeout(() => {
  console.log("stop");
}, 5000);

console.log("running");
