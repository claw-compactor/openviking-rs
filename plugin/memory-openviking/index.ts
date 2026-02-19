import type { OpenClawPluginApi } from "openclaw/plugin-sdk";
import { emptyPluginConfigSchema } from "openclaw/plugin-sdk";
import { Type } from "@sinclair/typebox";
import path from "node:path";
import fs from "node:fs/promises";

let ovModule: any = null;

const MemorySearchSchema = Type.Object({
  query: Type.String(),
  maxResults: Type.Optional(Type.Number()),
  minScore: Type.Optional(Type.Number()),
});

const MemoryGetSchema = Type.Object({
  path: Type.String(),
  from: Type.Optional(Type.Number()),
  lines: Type.Optional(Type.Number()),
});

/** Match OpenClaw's expected tool result format */
function jsonResult(payload: any) {
  return {
    content: [{ type: "text", text: JSON.stringify(payload, null, 2) }],
    details: payload,
  };
}

function getPluginDir() {
  try {
    return new URL('.', import.meta.url).pathname;
  } catch {
    return __dirname;
  }
}

function loadOpenVikingModule(pluginDir: string) {
  if (!ovModule) {
    const nodePath = path.join(pluginDir, "openviking-engine.darwin-arm64.node");
    ovModule = require(nodePath);
  }
  return ovModule;
}

function getWorkspaceDir(): string {
  const homeDir = process.env.HOME || require('os').homedir();
  return path.join(homeDir, '.openclaw', 'workspace');
}

async function fileBasedSearch(query: string, workspaceDir: string, maxResults: number = 10) {
  const results: any[] = [];
  const lowerQuery = query.toLowerCase();
  const queryWords = lowerQuery.split(/\s+/).filter(w => w.length > 2);

  async function searchFile(filePath: string, displayPath: string) {
    try {
      const content = await fs.readFile(filePath, 'utf-8');
      const lines = content.split('\n');
      lines.forEach((line, index) => {
        const lowerLine = line.toLowerCase();
        // Score by word match count
        const matchCount = queryWords.filter(w => lowerLine.includes(w)).length;
        if (matchCount > 0) {
          const score = Math.min(0.95, 0.3 + (matchCount / queryWords.length) * 0.6);
          results.push({
            text: line.trim(),
            path: displayPath,
            score,
            lines: `${index + 1}:${index + 1}`,
            context: lines.slice(Math.max(0, index - 1), index + 2).join('\n')
          });
        }
      });
    } catch {}
  }

  // Search MEMORY.md
  await searchFile(path.join(workspaceDir, "MEMORY.md"), "MEMORY.md");

  // Search memory/*.md
  try {
    const memDir = path.join(workspaceDir, "memory");
    const files = await fs.readdir(memDir);
    for (const file of files) {
      if (file.endsWith('.md')) {
        await searchFile(path.join(memDir, file), `memory/${file}`);
      }
    }
  } catch {}

  // Sort by score desc, return top N
  results.sort((a, b) => b.score - a.score);
  return results.slice(0, maxResults);
}

const memoryOpenVikingPlugin = {
  id: "memory-openviking",
  name: "Memory (OpenViking)",
  description: "OpenViking-rs based memory search with Rust native performance",
  kind: "memory" as const,
  configSchema: emptyPluginConfigSchema(),

  register(api: OpenClawPluginApi) {
    const pluginDir = getPluginDir();
    let ovLoaded = false;

    try {
      loadOpenVikingModule(pluginDir);
      ovLoaded = true;
      api.logger.info("memory-openviking: OpenViking-rs native module loaded");
    } catch (err) {
      api.logger.warn(`memory-openviking: native module not available, using file-based fallback: ${err}`);
    }

    api.registerTool(
      (ctx) => {
        const memorySearchTool = {
          label: "Memory Search",
          name: "memory_search",
          description: "Mandatory recall step: semantically search MEMORY.md + memory/*.md (and optional session transcripts) before answering questions about prior work, decisions, dates, people, preferences, or todos; returns top snippets with path + lines.",
          parameters: MemorySearchSchema,
          execute: async (_toolCallId: string, params: any) => {
            const query = params.query as string;
            const maxResults = params.maxResults || 10;
            const minScore = params.minScore || 0.1;

            let results: any[] = [];
            let provider = "openviking-rs";
            let fallback = false;

            if (ovLoaded) {
              try {
                const ov = loadOpenVikingModule(pluginDir);
                const ovResults = ov.searchMemory(query, null, null, maxResults);
                results = (ovResults || []).map((entry: any) => ({
                  text: entry.content || entry.text || "",
                  path: entry.path || `memory/openviking-${entry.id}.md`,
                  score: entry.score || 0.9,
                  lines: entry.lines || "1:10",
                  context: entry.overview || entry.content || ""
                }));
                api.logger.info(`memory-openviking: native search returned ${results.length} results`);
              } catch (err) {
                api.logger.warn(`memory-openviking: native search failed: ${err}`);
                fallback = true;
                provider = "file-based";
              }
            } else {
              fallback = true;
              provider = "file-based";
            }

            // File-based fallback
            if (results.length === 0) {
              const workspaceDir = getWorkspaceDir();
              results = await fileBasedSearch(query, workspaceDir, maxResults);
              if (results.length > 0) {
                fallback = true;
                provider = results.length > 0 ? "file-based" : provider;
              }
            }

            results = results.filter(r => r.score >= minScore);

            return jsonResult({
              results,
              provider,
              model: "openviking-rs",
              fallback,
              citations: "off",
              mode: "hybrid"
            });
          }
        };

        const memoryGetTool = {
          label: "Memory Get",
          name: "memory_get",
          description: "Safe snippet read from MEMORY.md or memory/*.md with optional from/lines; use after memory_search to pull only the needed lines and keep context small.",
          parameters: MemoryGetSchema,
          execute: async (_toolCallId: string, params: any) => {
            const relPath = params.path as string;
            const from = params.from;
            const lines = params.lines;

            try {
              const workspaceDir = getWorkspaceDir();
              const fullPath = path.join(workspaceDir, relPath);

              // Security: prevent path traversal
              const resolved = path.resolve(fullPath);
              if (!resolved.startsWith(path.resolve(workspaceDir))) {
                return jsonResult({ path: relPath, text: "", error: "path traversal blocked" });
              }

              const content = await fs.readFile(fullPath, 'utf-8');
              const allLines = content.split('\n');

              let selectedLines = allLines;
              if (from !== undefined) {
                const startIndex = Math.max(0, from - 1);
                const endIndex = lines !== undefined ? startIndex + lines : allLines.length;
                selectedLines = allLines.slice(startIndex, endIndex);
              }

              return jsonResult({
                path: relPath,
                text: selectedLines.join('\n'),
                lines: selectedLines.length,
                total: allLines.length
              });
            } catch (error) {
              return jsonResult({
                path: relPath,
                text: "",
                error: error instanceof Error ? error.message : String(error)
              });
            }
          }
        };

        return [memorySearchTool, memoryGetTool];
      },
      { names: ["memory_search", "memory_get"] }
    );

    // Register CLI override
    api.registerCli(
      ({ program }) => {
        const memory = program.command("memory").description("OpenViking-rs memory engine");

        memory.command("status").description("Show OpenViking-rs memory status").action(async () => {
          console.log("Memory Search (OpenViking-rs)");
          console.log(`Provider: ${ovLoaded ? "openviking-rs native" : "file-based fallback"}`);
          console.log(`Plugin: memory-openviking (loaded)`);
          console.log(`Native module: ${ovLoaded ? "✅ loaded" : "❌ not available"}`);
          console.log(`Module path: ${path.join(pluginDir, "openviking-engine.darwin-arm64.node")}`);
          console.log(`Workspace: ${getWorkspaceDir()}`);

          if (ovLoaded) {
            try {
              const ov = loadOpenVikingModule(pluginDir);
              const info = ov.getEngineInfo?.() || {};
              console.log(`Engine version: ${info.version || "unknown"}`);
              console.log(`Crates: ${info.crates || "9"}`);
            } catch {}
          }
        });

        memory.command("search").description("Search memory").argument("<query>").action(async (query: string) => {
          const workspaceDir = getWorkspaceDir();
          const results = await fileBasedSearch(query, workspaceDir, 10);
          if (results.length === 0) {
            console.log("No results found.");
            return;
          }
          for (const r of results) {
            console.log(`[${r.score.toFixed(2)}] ${r.path}#${r.lines}: ${r.text}`);
          }
        });
      },
      { commands: ["memory"] }
    );

    api.logger.info("memory-openviking: plugin registered with hybrid OpenViking-rs + file-based search");
  }
};

export default memoryOpenVikingPlugin;
