'use client';

import { useState } from 'react';
import SolanaChart from '@/components/SolanaChart';
import OrderPanel from '@/components/OrderPanel';
import OrderBook from '@/components/OrderBook';
import TradeHistory from '@/components/TradeHistory';
import AssetList from '@/components/AssetList';

export default function Home() {
  const [selectedMarket] = useState('SOL/USDT');

  return (
    <main className="min-h-screen bg-gray-900">
      <div className="flex h-[calc(100vh-4rem)] max-w-[1920px] mx-auto">
        {/* 좌측: 자산 리스트 */}
        <AssetList />

        {/* 중앙: 차트 + 주문 패널 */}
        <div className="flex-1 flex flex-col p-6 gap-6 overflow-hidden">
          {/* 차트 영역 */}
          <div className="flex-[1.5] min-h-0">
            <div className="mb-3">
              <h2 className="text-lg font-semibold text-white">{selectedMarket}</h2>
            </div>
            <div className="h-full">
              <SolanaChart />
            </div>
          </div>

          {/* 주문 패널 */}
          <div className="flex-[1] min-h-0 flex-shrink-0">
            <OrderPanel />
          </div>
        </div>

        {/* 우측: 호가창 + 체결 내역 */}
        <div className="w-72 flex-shrink-0 p-6 flex flex-col gap-6 overflow-hidden">
          <div className="flex-1 min-h-0">
            <OrderBook />
          </div>
          <div className="h-56">
            <TradeHistory />
          </div>
        </div>
      </div>
    </main>
  );
}

