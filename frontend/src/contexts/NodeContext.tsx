import {
  createContext,
  useCallback,
  useContext,
  useState,
  type ReactNode,
} from "react";
import { rpcClient } from "../services";

export interface NodeOption {
  id: string;
  label: string;
  url: string;
}

// "local" honors VITE_RPC_URL (same fallback as the rpc client); the numbered
// entries match the simulation containers published by docker-compose.sim.yml
// (node1 -> 7101 .. node4 -> 7104).
const LOCAL_URL: string =
  import.meta.env.VITE_RPC_URL ?? "http://localhost:7001/rpc";

const NODE_OPTIONS: NodeOption[] = [
  { id: "local", label: "local", url: LOCAL_URL },
  { id: "node1", label: "node1 (7101)", url: "http://localhost:7101/rpc" },
  { id: "node2", label: "node2 (7102)", url: "http://localhost:7102/rpc" },
  { id: "node3", label: "node3 (7103)", url: "http://localhost:7103/rpc" },
  { id: "node4", label: "node4 (7104)", url: "http://localhost:7104/rpc" },
];

const STORAGE_KEY = "caramuru.selectedNode";

interface NodeContextType {
  nodes: NodeOption[];
  selectedNode: NodeOption;
  selectNode: (id: string) => void;
}

const NodeContext = createContext<NodeContextType | null>(null);

function resolveInitial(): NodeOption {
  const saved =
    typeof localStorage !== "undefined" ? localStorage.getItem(STORAGE_KEY) : null;
  return NODE_OPTIONS.find((n) => n.id === saved) ?? NODE_OPTIONS[0];
}

export function NodeProvider({ children }: { children: ReactNode }) {
  // Resolve the initial node during render (before any child effect fires) and
  // point the shared rpc client at it, so the first fetch already hits the
  // right daemon.
  const [selectedNode, setSelectedNode] = useState<NodeOption>(() => {
    const initial = resolveInitial();
    rpcClient.setBaseUrl(initial.url);
    return initial;
  });

  const selectNode = useCallback((id: string) => {
    const next = NODE_OPTIONS.find((n) => n.id === id);
    if (!next) return;
    // Switch the client synchronously, then re-render. App keys the routed tree
    // on selectedNode.id, so every page remounts and re-fetches from the new node.
    rpcClient.setBaseUrl(next.url);
    if (typeof localStorage !== "undefined") {
      localStorage.setItem(STORAGE_KEY, next.id);
    }
    setSelectedNode(next);
  }, []);

  return (
    <NodeContext.Provider value={{ nodes: NODE_OPTIONS, selectedNode, selectNode }}>
      {children}
    </NodeContext.Provider>
  );
}

export function useNode() {
  const ctx = useContext(NodeContext);
  if (!ctx) {
    throw new Error("useNode must be used within a NodeProvider");
  }
  return ctx;
}
