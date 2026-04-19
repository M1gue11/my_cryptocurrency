import type { ReactNode } from "react";
import { useEffect, useState } from "react";
import { NavLink, useLocation } from "react-router-dom";
import { rpcClient } from "../services";
import { ConsolePill } from "./Console";

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  const location = useLocation();
  const [nodeOnline, setNodeOnline] = useState(true);
  const [height, setHeight] = useState<number | null>(null);
  const [miningState, setMiningState] = useState<"mining" | "idle">("idle");

  useEffect(() => {
    let cancelled = false;

    const fetchStatus = async () => {
      try {
        const [nodeStatus, miningInfo] = await Promise.all([
          rpcClient.nodeStatus(),
          rpcClient.mineInfo().catch(() => null),
        ]);

        if (cancelled) return;

        setNodeOnline(true);
        setHeight(nodeStatus.block_height);
        setMiningState(miningInfo?.is_currently_mining ? "mining" : "idle");
      } catch {
        if (cancelled) return;
        setNodeOnline(false);
        setMiningState("idle");
      }
    };

    void fetchStatus();
    const interval = setInterval(fetchStatus, 10000);
    return () => {
      cancelled = true;
      clearInterval(interval);
    };
  }, []);

  const footerLabel = (() => {
    if (location.pathname === "/blocks") return "blockchain explorer";
    if (location.pathname === "/transactions") return "transaction workbench";
    if (location.pathname === "/wallet") return "wallet console";
    if (location.pathname === "/network") return "runtime and peers";
    if (location.pathname === "/mining") return "mining center";
    if (location.pathname === "/logs") return "diagnostics";
    return "operator overview";
  })();

  return (
    <div className="crm-shell">
      <header className="crm-topbar">
        <div className="mx-auto flex w-full max-w-360 flex-col gap-3 px-4 py-3 lg:flex-row lg:items-center lg:gap-6">
          <div className="flex items-center gap-2">
            <span className="crm-logo-mark" />
            <div className="text-[13px] font-semibold tracking-[-0.02em] text-(--crm-fg)">
              caramuru
            </div>
            <div className="crm-mono text-[10.5px] text-(--crm-dim)">
              v0.3.1
            </div>
          </div>

          <nav className="flex min-w-0 flex-1 gap-1 overflow-x-auto pb-1 lg:pb-0">
            <NavItem to="/">dashboard</NavItem>
            <NavItem to="/blocks">blockchain</NavItem>
            <NavItem to="/transactions">transactions</NavItem>
            <NavItem to="/wallet">wallet</NavItem>
            <NavItem to="/mining">mining</NavItem>
            <NavItem to="/network">node</NavItem>
            <NavItem to="/logs">logs</NavItem>
          </nav>

          <div className="flex flex-wrap items-center gap-2 text-[11px] text-[var(--crm-dim)] lg:justify-end">
            <span className="crm-mono">rpc http://localhost:7001/rpc</span>
            <ConsolePill tone={nodeOnline ? "accent" : "warn"} dot>
              {nodeOnline ? "live" : "offline"}
            </ConsolePill>
            <ConsolePill>
              {height !== null ? `#${height.toLocaleString()}` : "-"}
            </ConsolePill>
            <ConsolePill>{miningState}</ConsolePill>
          </div>
        </div>
      </header>

      <main className="crm-main">{children}</main>

      <footer className="crm-footer">
        <div className="mx-auto flex max-w-[1440px] flex-wrap items-center gap-x-3 gap-y-1 px-4 py-2">
          <span>local operator panel</span>
          <span>.</span>
          <span>{nodeOnline ? "node healthy" : "daemon unreachable"}</span>
          <span>.</span>
          <span>{footerLabel}</span>
          <span className="ml-auto">caramuru . pow node . live rpc data</span>
        </div>
      </footer>
    </div>
  );
}

function NavItem({ to, children }: { to: string; children: ReactNode }) {
  return (
    <NavLink
      to={to}
      className={({ isActive }) =>
        isActive ? "crm-nav-link crm-nav-link--active" : "crm-nav-link"
      }
    >
      {children}
    </NavLink>
  );
}
