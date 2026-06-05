'use client'

import { useState, useEffect, useCallback, useRef } from "react";

const API_BASE = process.env.NEXT_PUBLIC_API_URL || "";
const TREASURY_ADDRESS = process.env.NEXT_PUBLIC_TREASURY_ADDRESS || "";
const WS_URL = API_BASE.replace(/^http(s?):\/\//, 'ws$1://') + '/ws';

interface Deposit {
  id: number;
  user_address: string;
  deposit_address: string;
  salt: string;
  status: string;
  balance: string;
}

export default function Home() {
  const [deposits, setDeposits] = useState<Deposit[]>([]);
  const [userAddress, setUserAddress] = useState("0xabc...");
  const [loading, setLoading] = useState(false);
  const [routing, setRouting] = useState(false);
  const [treasuryBalance, setTreasuryBalance] = useState("0.0000");
  const [error, setError] = useState("");
  const routedRef = useRef<Set<string>>(new Set());
  const wsRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | undefined>(undefined);

  const connectWs = useCallback(() => {
    if (wsRef.current?.readyState === WebSocket.OPEN) return;

    const ws = new WebSocket(WS_URL);
    wsRef.current = ws;

    ws.onmessage = (event) => {
      const data = JSON.parse(event.data);
      setTreasuryBalance(data.treasury_balance);
      setDeposits((prev) =>
        prev.map((d) => {
          const balance = data.balances[d.deposit_address] ?? d.balance;
          const bal = parseFloat(balance);
          let status = d.status;
          if (routedRef.current.has(d.deposit_address) || d.status === "routed") {
            status = "routed";
          } else if (bal > 0) {
            status = "funded";
          } else {
            status = "pending";
          }
          return { ...d, balance, status };
        })
      );
    };

    ws.onclose = () => {
      wsRef.current = null;
      reconnectTimerRef.current = setTimeout(connectWs, 3000);
    };

    ws.onerror = () => {
      ws.close();
    };
  }, []);

  useEffect(() => {
    const saved = localStorage.getItem("deposits");
    if (saved) {
      try { setDeposits(JSON.parse(saved)); } catch { /* ignore */ }
    }
    fetch(`${API_BASE}/deposits`)
      .then((r) => r.json())
      .then((data) => {
        if (data.deposits?.length) {
          setDeposits(data.deposits.map((d: any, i: number) => ({
            id: i + 1,
            user_address: d.user_address,
            deposit_address: d.deposit_address,
            salt: d.salt,
            status: d.status,
            balance: d.balance ?? "0.0000",
          })));
        }
      })
      .catch(() => {});
  }, []);

  useEffect(() => {
    localStorage.setItem("deposits", JSON.stringify(deposits));
  }, [deposits]);

  useEffect(() => {
    connectWs();
    return () => {
      wsRef.current?.close();
      clearTimeout(reconnectTimerRef.current);
    };
  }, [connectWs]);

  const getNextDeposit = async () => {
    setLoading(true);
    setError("");
    try {
      const res = await fetch(`${API_BASE}/deposit`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ user: userAddress }),
      });
      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error || "Failed to get deposit address");
      }
      const data = await res.json();
      const newDeposit: Deposit = {
        id: deposits.length + 1,
        user_address: userAddress,
        deposit_address: data.deposit_address,
        salt: data.salt,
        status: "pending",
        balance: "0.0000",
      };
      setDeposits((prev) => [...prev, newDeposit]);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setLoading(false);
    }
  };

  const routeFunds = async () => {
    setRouting(true);
    setError("");
    try {
      const res = await fetch(`${API_BASE}/router`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: "{}",
      });
      if (!res.ok) {
        const err = await res.json();
        throw new Error(err.error || "Failed to route funds");
      }
      const data = await res.json();
      setDeposits(prev => {
        prev.forEach(d => routedRef.current.add(d.deposit_address));
        return prev.map(d => ({...d, status: "routed"}));
      });
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Unknown error");
    } finally {
      setRouting(false);
    }
  };

  return (
    <div className="min-h-screen bg-zinc-50 dark:bg-black p-6">
      <div className="max-w-5xl mx-auto">
        <h1 className="text-2xl font-bold mb-6 text-zinc-900 dark:text-zinc-100">
          Deposit Proxies on Sepolia
        </h1>

        <div className="bg-white dark:bg-zinc-900 rounded-xl shadow-sm border border-zinc-200 dark:border-zinc-800 p-6 mb-6">
          <div className="flex flex-wrap items-end gap-4">
            <div className="flex-1 min-w-[280px]">
              <label className="block text-sm font-medium text-zinc-600 dark:text-zinc-400 mb-1">
                User Address
              </label>
              <input
                type="text"
                value={userAddress}
                onChange={(e) => setUserAddress(e.target.value)}
                placeholder="0x..."
                className="w-full px-3 py-2 rounded-lg border border-zinc-300 dark:border-zinc-700 bg-white dark:bg-zinc-800 text-zinc-900 dark:text-zinc-100 text-sm font-mono focus:outline-none focus:ring-2 focus:ring-blue-500"
              />
            </div>
            <button
              onClick={getNextDeposit}
              disabled={loading}
              className="px-5 py-2 rounded-lg bg-blue-600 hover:bg-blue-700 disabled:bg-blue-400 text-white font-medium text-sm transition-colors"
            >
              {loading ? "Generating..." : "Get Next Deposit Address"}
            </button>
            <button
              onClick={routeFunds}
              disabled={routing}
              className="px-5 py-2 rounded-lg bg-emerald-600 hover:bg-emerald-700 disabled:bg-emerald-400 text-white font-medium text-sm transition-colors"
            >
              {routing ? "Routing..." : "Route Funds to Treasury"}
            </button>
          </div>
          {error && (
            <p className="mt-3 text-sm text-red-600 dark:text-red-400">{error}</p>
          )}
        </div>

        <div className="bg-white dark:bg-zinc-900 rounded-xl shadow-sm border border-zinc-200 dark:border-zinc-800 p-6 mb-6">
          <div className="flex items-center justify-between">
            <div>
              <h2 className="text-sm font-semibold text-zinc-600 dark:text-zinc-400 uppercase tracking-wider">
                Treasury
              </h2>
              <p className="mt-1 font-mono text-xs text-zinc-500 dark:text-zinc-500">
                {TREASURY_ADDRESS.slice(0, 6)}...{TREASURY_ADDRESS.slice(-4)}
              </p>
            </div>
            <div className="text-right">
              <p className="text-2xl font-bold text-zinc-900 dark:text-zinc-100">
                {parseFloat(treasuryBalance) > 0 ? (
                  <span className="text-emerald-600 dark:text-emerald-400">
                    {treasuryBalance}
                  </span>
                ) : (
                  treasuryBalance
                )}
              </p>
              <p className="text-xs text-zinc-400 dark:text-zinc-500">ETH</p>
            </div>
          </div>
          <div className="mt-3 pt-3 border-t border-zinc-100 dark:border-zinc-800">
            <p className="text-xs font-mono text-zinc-400 dark:text-zinc-600 break-all select-all">
              {TREASURY_ADDRESS}
            </p>
          </div>
        </div>

        <div className="bg-white dark:bg-zinc-900 rounded-xl shadow-sm border border-zinc-200 dark:border-zinc-800 overflow-hidden">
          <div className="overflow-x-auto">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-zinc-200 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-800/50">
                  <th className="text-left px-4 py-3 font-medium text-zinc-500 dark:text-zinc-400 w-12">
                    #
                  </th>
                  <th className="text-left px-4 py-3 font-medium text-zinc-500 dark:text-zinc-400">
                    Deposit Address
                  </th>
                  <th className="text-left px-4 py-3 font-medium text-zinc-500 dark:text-zinc-400">
                    Status
                  </th>
                  <th className="text-right px-4 py-3 font-medium text-zinc-500 dark:text-zinc-400">
                    Last Balance (ETH)
                  </th>
                  <th className="text-left px-4 py-3 font-medium text-zinc-500 dark:text-zinc-400">
                    Actions
                  </th>
                </tr>
              </thead>
              <tbody>
                {deposits.length === 0 ? (
                  <tr>
                    <td
                      colSpan={5}
                      className="px-4 py-12 text-center text-zinc-400 dark:text-zinc-600"
                    >
                      No deposit addresses yet. Click &quot;Get Next Deposit Address&quot; to
                      start.
                    </td>
                  </tr>
                ) : (
                  deposits.map((dep) => {
                    const bal = parseFloat(dep.balance);
                    return (
                      <tr
                        key={dep.deposit_address}
                        className="border-b border-zinc-100 dark:border-zinc-800 hover:bg-zinc-50 dark:hover:bg-zinc-800/30 transition-colors"
                      >
                        <td className="px-4 py-3 text-zinc-500">{dep.id}</td>
                        <td className="px-4 py-3 font-mono text-xs text-zinc-800 dark:text-zinc-200">
                          {dep.deposit_address}
                        </td>
                        <td className="px-4 py-3">
                          <span
                            className={`inline-flex items-center px-2 py-0.5 rounded-full text-xs font-medium ${
                              dep.status === "routed"
                                ? "bg-green-100 text-green-700 dark:bg-green-900/30 dark:text-green-400"
                                : dep.status === "funded"
                                  ? "bg-blue-100 text-blue-700 dark:bg-blue-900/30 dark:text-blue-400"
                                  : "bg-zinc-100 text-zinc-600 dark:bg-zinc-800 dark:text-zinc-400"
                            }`}
                          >
                            {dep.status === "routed"
                              ? "Routed"
                              : dep.status === "funded"
                                ? "Funded"
                                : "Pending"}
                          </span>
                        </td>
                        <td className="px-4 py-3 text-right font-mono text-sm text-zinc-800 dark:text-zinc-200">
                          {bal > 0 ? (
                            <span className="text-emerald-600 dark:text-emerald-400">
                              {dep.balance}
                            </span>
                          ) : (
                            dep.balance
                          )}
                        </td>
                        <td className="px-4 py-3">
                            <button
                              onClick={async () => {
                                setError("");
                                try {
                                  const res = await fetch(`${API_BASE}/router`, {
                                    method: "POST",
                                    headers: {
                                      "Content-Type": "application/json",
                                    },
                                    body: JSON.stringify({
                                      deposit_address: dep.deposit_address,
                                    }),
                                  });
                                  if (!res.ok) {
                                    const err = await res.json();
                                    throw new Error(
                                      err.error || "Route failed"
                                    );
                                  }
                                  routedRef.current.add(dep.deposit_address);
                                  setDeposits(prev => prev.map(d =>
                                    d.deposit_address === dep.deposit_address
                                      ? { ...d, status: "routed" }
                                      : d
                                  ));
                                } catch (e: unknown) {
                                  setError(
                                    e instanceof Error
                                      ? e.message
                                      : "Route failed"
                                  );
                                }
                              }}
                              disabled={dep.status === "routed" || bal <= 0}
                              className="px-2.5 py-1 rounded text-xs font-medium transition-colors disabled:opacity-40 disabled:cursor-not-allowed enabled:bg-zinc-100 enabled:hover:bg-zinc-200 dark:enabled:bg-zinc-800 dark:enabled:hover:bg-zinc-700 text-zinc-700 dark:text-zinc-300"
                            >
                              {dep.status === "routed" ? "Routed" : "Route"}
                            </button>
                        </td>
                      </tr>
                    );
                  })
                )}
              </tbody>
            </table>
          </div>
          {deposits.length > 0 && (
            <div className="px-4 py-2 text-xs text-zinc-400 dark:text-zinc-600 border-t border-zinc-100 dark:border-zinc-800 bg-zinc-50 dark:bg-zinc-800/30">
              Live balance updates via WebSocket
            </div>
          )}
        </div>
      </div>
    </div>
  );
}
