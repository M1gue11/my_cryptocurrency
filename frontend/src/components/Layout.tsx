import type { ReactNode } from 'react';

interface LayoutProps {
  children: ReactNode;
}

export function Layout({ children }: LayoutProps) {
  return (
    <div className="min-h-screen bg-gray-900 text-gray-100">
      {/* Header */}
      <header className="bg-gray-800 border-b border-gray-700">
        <div className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8">
          <div className="flex items-center justify-between h-16">
            <div className="flex items-center gap-3">
              <span className="text-2xl">🔗</span>
              <h1 className="text-xl font-bold text-white">Caramuru</h1>
            </div>
            <nav className="flex gap-6">
              <NavLink href="/">Dashboard</NavLink>
              <NavLink href="/blocks">Blocks</NavLink>
              <NavLink href="/wallet">Wallet</NavLink>
              <NavLink href="/transactions">Transactions</NavLink>
            </nav>
          </div>
        </div>
      </header>

      {/* Main Content */}
      <main className="max-w-7xl mx-auto px-4 sm:px-6 lg:px-8 py-8">
        {children}
      </main>
    </div>
  );
}

function NavLink({ href, children }: { href: string; children: ReactNode }) {
  // Simple link for now - can be replaced with React Router later
  const isActive = window.location.pathname === href;

  return (
    <a
      href={href}
      className={`px-3 py-2 rounded-md text-sm font-medium transition-colors ${
        isActive
          ? 'bg-gray-900 text-white'
          : 'text-gray-300 hover:bg-gray-700 hover:text-white'
      }`}
    >
      {children}
    </a>
  );
}
