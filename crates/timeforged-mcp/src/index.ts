#!/usr/bin/env node
import { McpServer } from "@modelcontextprotocol/sdk/server/mcp.js";
import { StdioServerTransport } from "@modelcontextprotocol/sdk/server/stdio.js";
import { z } from "zod";

const SERVER_URL = process.env.TF_SERVER_URL || "http://127.0.0.1:6175";
const API_KEY = process.env.TF_API_KEY || "";

if (!API_KEY) {
  console.error(
    "Warning: TF_API_KEY not set. Requests will be sent without authentication."
  );
}

async function tfFetch(
  path: string,
  options: RequestInit = {}
): Promise<unknown> {
  const url = `${SERVER_URL}${path}`;
  const headers: Record<string, string> = {
    "Content-Type": "application/json",
    ...(API_KEY ? { "X-Api-Key": API_KEY } : {}),
    ...(options.headers as Record<string, string> | undefined),
  };

  const resp = await fetch(url, {
    ...options,
    headers,
    signal: AbortSignal.timeout(10_000),
  });

  const body = await resp.text();
  if (!resp.ok) {
    throw new Error(`HTTP ${resp.status}: ${body}`);
  }

  try {
    return JSON.parse(body);
  } catch {
    return body;
  }
}

function formatSeconds(totalSeconds: number): string {
  const h = Math.floor(totalSeconds / 3600);
  const m = Math.floor((totalSeconds % 3600) / 60);
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m`;
}

const server = new McpServer({
  name: "timeforged",
  version: "0.1.0",
});

// --- tf_status ---
server.tool("tf_status", "Check TimeForged daemon status", {}, async () => {
  try {
    const data = (await tfFetch("/api/v1/status")) as {
      status: string;
      version: string;
      user_count: number;
      event_count: number;
    };
    return {
      content: [
        {
          type: "text" as const,
          text: [
            `Status: ${data.status}`,
            `Version: ${data.version}`,
            `Users: ${data.user_count}`,
            `Events: ${data.event_count}`,
          ].join("\n"),
        },
      ],
    };
  } catch (e) {
    return {
      content: [
        { type: "text" as const, text: `Error: ${(e as Error).message}` },
      ],
      isError: true,
    };
  }
});

// --- tf_today ---
server.tool("tf_today", "Get today's coding time summary", {}, async () => {
  try {
    const now = new Date();
    const startOfDay = new Date(
      now.getFullYear(),
      now.getMonth(),
      now.getDate()
    );
    const from = startOfDay.toISOString();
    const to = now.toISOString();

    const data = (await tfFetch(
      `/api/v1/reports/summary?from=${encodeURIComponent(from)}&to=${encodeURIComponent(to)}`
    )) as {
      total_seconds: number;
      projects: Array<{
        name: string;
        total_seconds: number;
        percent: number;
      }>;
      languages: Array<{
        name: string;
        total_seconds: number;
        percent: number;
      }>;
    };

    const lines: string[] = [`Today: ${formatSeconds(data.total_seconds)}`];

    if (data.projects.length > 0) {
      lines.push(
        "",
        "Projects:",
        ...data.projects.map(
          (p) =>
            `  ${p.name}: ${formatSeconds(p.total_seconds)} (${p.percent.toFixed(0)}%)`
        )
      );
    }

    if (data.languages.length > 0) {
      lines.push(
        "",
        "Languages:",
        ...data.languages.map(
          (l) =>
            `  ${l.name}: ${formatSeconds(l.total_seconds)} (${l.percent.toFixed(0)}%)`
        )
      );
    }

    return {
      content: [{ type: "text" as const, text: lines.join("\n") }],
    };
  } catch (e) {
    return {
      content: [
        { type: "text" as const, text: `Error: ${(e as Error).message}` },
      ],
      isError: true,
    };
  }
});

// --- tf_report ---
server.tool(
  "tf_report",
  "Get coding time summary for a date range",
  {
    from: z
      .string()
      .optional()
      .describe("Start datetime (ISO 8601). Defaults to 7 days ago."),
    to: z
      .string()
      .optional()
      .describe("End datetime (ISO 8601). Defaults to now."),
    project: z.string().optional().describe("Filter by project name"),
    language: z.string().optional().describe("Filter by language"),
  },
  async ({ from, to, project, language }) => {
    try {
      const params = new URLSearchParams();
      if (from) params.set("from", from);
      if (to) params.set("to", to);
      if (project) params.set("project", project);
      if (language) params.set("language", language);

      const qs = params.toString();
      const data = (await tfFetch(
        `/api/v1/reports/summary${qs ? `?${qs}` : ""}`
      )) as {
        total_seconds: number;
        from: string;
        to: string;
        projects: Array<{
          name: string;
          total_seconds: number;
          percent: number;
        }>;
        languages: Array<{
          name: string;
          total_seconds: number;
          percent: number;
        }>;
        days: Array<{ date: string; total_seconds: number }>;
      };

      const lines: string[] = [
        `Total: ${formatSeconds(data.total_seconds)}`,
        `Period: ${data.from} → ${data.to}`,
      ];

      if (data.projects.length > 0) {
        lines.push(
          "",
          "Projects:",
          ...data.projects.map(
            (p) =>
              `  ${p.name}: ${formatSeconds(p.total_seconds)} (${p.percent.toFixed(0)}%)`
          )
        );
      }

      if (data.languages.length > 0) {
        lines.push(
          "",
          "Languages:",
          ...data.languages.map(
            (l) =>
              `  ${l.name}: ${formatSeconds(l.total_seconds)} (${l.percent.toFixed(0)}%)`
          )
        );
      }

      if (data.days.length > 0) {
        lines.push(
          "",
          "Daily:",
          ...data.days.map(
            (d) => `  ${d.date}: ${formatSeconds(d.total_seconds)}`
          )
        );
      }

      return {
        content: [{ type: "text" as const, text: lines.join("\n") }],
      };
    } catch (e) {
      return {
        content: [
          { type: "text" as const, text: `Error: ${(e as Error).message}` },
        ],
        isError: true,
      };
    }
  }
);

// --- tf_sessions ---
server.tool(
  "tf_sessions",
  "List coding sessions",
  {
    from: z.string().optional().describe("Start datetime (ISO 8601)"),
    to: z.string().optional().describe("End datetime (ISO 8601)"),
    project: z.string().optional().describe("Filter by project name"),
  },
  async ({ from, to, project }) => {
    try {
      const params = new URLSearchParams();
      if (from) params.set("from", from);
      if (to) params.set("to", to);
      if (project) params.set("project", project);

      const qs = params.toString();
      const data = (await tfFetch(
        `/api/v1/reports/sessions${qs ? `?${qs}` : ""}`
      )) as Array<{
        start: string;
        end: string;
        duration_seconds: number;
        project: string | null;
        event_count: number;
      }>;

      if (data.length === 0) {
        return {
          content: [{ type: "text" as const, text: "No sessions found." }],
        };
      }

      const lines = data.map((s, i) => {
        const proj = s.project ? ` [${s.project}]` : "";
        return `${i + 1}. ${s.start} → ${s.end} (${formatSeconds(s.duration_seconds)}, ${s.event_count} events)${proj}`;
      });

      return {
        content: [
          {
            type: "text" as const,
            text: `Sessions (${data.length}):\n${lines.join("\n")}`,
          },
        ],
      };
    } catch (e) {
      return {
        content: [
          { type: "text" as const, text: `Error: ${(e as Error).message}` },
        ],
        isError: true,
      };
    }
  }
);

// --- tf_send ---
server.tool(
  "tf_send",
  "Send a manual event/heartbeat to TimeForged",
  {
    entity: z.string().describe("File path or entity name"),
    event_type: z
      .enum(["file", "terminal", "browser", "meeting", "custom"])
      .optional()
      .default("file")
      .describe("Event type"),
    project: z.string().optional().describe("Project name"),
    language: z.string().optional().describe("Language"),
    activity: z
      .enum([
        "coding",
        "browsing",
        "debugging",
        "building",
        "communicating",
        "designing",
        "other",
      ])
      .optional()
      .default("coding")
      .describe("Activity type"),
  },
  async ({ entity, event_type, project, language, activity }) => {
    try {
      const payload = {
        timestamp: new Date().toISOString(),
        event_type,
        entity,
        activity,
        ...(project ? { project } : {}),
        ...(language ? { language } : {}),
        metadata: { source: "mcp" },
      };

      const data = (await tfFetch("/api/v1/events", {
        method: "POST",
        body: JSON.stringify(payload),
      })) as {
        id: number;
        timestamp: string;
        event_type: string;
        entity: string;
      };

      return {
        content: [
          {
            type: "text" as const,
            text: `Event sent: id=${data.id}, entity=${data.entity}, type=${data.event_type}`,
          },
        ],
      };
    } catch (e) {
      return {
        content: [
          { type: "text" as const, text: `Error: ${(e as Error).message}` },
        ],
        isError: true,
      };
    }
  }
);

// --- Start server ---
async function main() {
  const transport = new StdioServerTransport();
  await server.connect(transport);
}

main().catch((e) => {
  console.error("Fatal:", e);
  process.exit(1);
});
