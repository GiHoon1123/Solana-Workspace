'use client';

import { useState } from 'react';
import MyOrders from './MyOrders';

type TabType = 'pending' | 'filled';

export default function OrderManagement() {
  const [activeTab, setActiveTab] = useState<TabType>('pending');

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 p-5 h-full flex flex-col overflow-hidden">
      {/* 탭 헤더 */}
      <div className="flex mb-4 flex-shrink-0">
        <button
          onClick={() => setActiveTab('pending')}
          className={`flex-1 py-2 px-4 rounded-l font-semibold text-sm transition-colors ${
            activeTab === 'pending'
              ? 'bg-gray-700 text-white'
              : 'bg-gray-900 text-gray-400 hover:bg-gray-800'
          }`}
        >
          미체결
        </button>
        <button
          onClick={() => setActiveTab('filled')}
          className={`flex-1 py-2 px-4 rounded-r font-semibold text-sm transition-colors ${
            activeTab === 'filled'
              ? 'bg-gray-700 text-white'
              : 'bg-gray-900 text-gray-400 hover:bg-gray-800'
          }`}
        >
          체결
        </button>
      </div>

      {/* 탭 컨텐츠 */}
      <div className="flex-1 min-h-0 overflow-hidden">
        <div className="h-full">
          <MyOrders filterStatus={activeTab === 'pending' ? ['pending', 'partial'] : ['filled']} />
        </div>
      </div>
    </div>
  );
}

