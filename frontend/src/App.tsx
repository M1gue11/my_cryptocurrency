import { BrowserRouter, Routes, Route, useLocation } from 'react-router-dom';
import { AnimatePresence } from 'motion/react';
import { Layout } from './components';
import { Dashboard, Blocks, Wallet, Transactions, Network, Logs, Mining } from './pages';
import { WalletProvider } from './contexts';

function AnimatedRoutes() {
  const location = useLocation();

  return (
    <AnimatePresence mode="wait">
      <Routes location={location} key={location.pathname}>
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
    <WalletProvider>
      <BrowserRouter>
        <Layout>
          <AnimatedRoutes />
        </Layout>
      </BrowserRouter>
    </WalletProvider>
  );
}

export default App;
