import fs from "node:fs";

const [templatePath, outputPath] = process.argv.slice(2);
if (!templatePath || !outputPath) {
  console.error("usage: node scripts/render-wrangler-config.mjs TEMPLATE OUTPUT");
  process.exit(1);
}

const required = ["WORKER_NAME", "D1_DATABASE_NAME", "D1_DATABASE_ID"];
for (const name of required) {
  if (!process.env[name]?.trim()) {
    console.error(`Missing required deployment variable: ${name}`);
    process.exit(1);
  }
}

const replacements = {
  "__WORKER_NAME__": process.env.WORKER_NAME.trim(),
  "__D1_DATABASE_NAME__": process.env.D1_DATABASE_NAME.trim(),
  "__D1_DATABASE_ID__": process.env.D1_DATABASE_ID.trim(),
};
let content = fs.readFileSync(templatePath, "utf8");
for (const [placeholder, value] of Object.entries(replacements)) {
  content = content.replaceAll(placeholder, value.replaceAll("\\", "\\\\").replaceAll('"', '\\"'));
}
fs.writeFileSync(outputPath, content);
