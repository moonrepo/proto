console.log("start");

await new Promise((resolve) => {
  setTimeout(() => {
    console.log("running");
    resolve();
  }, 2500);
});

console.log("stop");
