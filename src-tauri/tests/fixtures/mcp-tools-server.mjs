import readline from "node:readline";

const input = readline.createInterface({ input: process.stdin });

input.on("line", (line) => {
  if (!line.trim()) return;
  const message = JSON.parse(line);
  if (message.method === "initialize") {
    process.stdout.write(`${JSON.stringify({
      jsonrpc: "2.0",
      id: message.id,
      result: {
        protocolVersion: message.params.protocolVersion,
        capabilities: { tools: { listChanged: false } },
        serverInfo: { name: "agentdock-test-server", version: "1.0.0" },
      },
    })}\n`);
    return;
  }
  if (message.method === "tools/list") {
    process.stdout.write(`${JSON.stringify({
      jsonrpc: "2.0",
      id: message.id,
      result: {
        tools: [{
          name: "search_notes",
          title: "Search notes",
          description: "Search notes by keyword.",
          inputSchema: {
            type: "object",
            properties: {
              query: { type: "string", description: "Search keyword" },
              limit: { type: "integer", default: 20 },
            },
            required: ["query"],
          },
        }],
      },
    })}\n`);
  }
});
