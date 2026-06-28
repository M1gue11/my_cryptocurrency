import { BrowserRouter, Routes, Route, useLocation } from 'react-router-dom';
import { AnimatePresence } from 'motion/react';
import { Layout } from './components';
import { Dashboard, Blocks, Wallet, Transactions, Network, Logs, Mining } from './pages';
import { WalletProvider, NodeProvider, useNode } from './contexts';

function AnimatedRoutes() {
  const location = useLocation();
  const { selectedNode } = useNode();

  return (
    <AnimatePresence mode="wait">
      {/* Keying on the node id remounts every page when the node changes,
          so each one re-fetches against the newly selected daemon. */}
      <Routes location={location} key={`${selectedNode.id}:${location.pathname}`}>
        <Route path="/" element={<Dashboard />} />
        <Route path="/blocks" element={<Blocks />} />
        <Route path="/wallet" element={<Wallet />} />
        <Route path="/transactions" element={<Transactions />} />
        <Route path="/network" element={<Network />} />
        <Route path="/logs" element={<Logs />} />
        <Route path="/mining" element={<Mining />} />
      </Routes>
    </AnimatePresence>
  );
}

function App() {
  return (
    <NodeProvider>
      <WalletProvider>
        <BrowserRouter>
          <Layout>
            <AnimatedRoutes />
          </Layout>
        </BrowserRouter>
      </WalletProvider>
    </NodeProvider>
  );
}

export default App;
