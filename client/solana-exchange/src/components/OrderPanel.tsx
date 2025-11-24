'use client';

import { useState } from 'react';

type OrderType = 'buy' | 'sell';

export default function OrderPanel() {
  const [orderType, setOrderType] = useState<OrderType>('buy');
  const [price, setPrice] = useState('');
  const [amount, setAmount] = useState('');
  const [total, setTotal] = useState('');

  const handlePriceChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setPrice(value);
    if (value && amount) {
      const priceNum = parseFloat(value);
      const amountNum = parseFloat(amount);
      if (!isNaN(priceNum) && !isNaN(amountNum) && priceNum > 0 && amountNum > 0) {
        setTotal((priceNum * amountNum).toFixed(2));
      } else {
        setTotal('');
      }
    } else {
      setTotal('');
    }
  };

  const handleAmountChange = (e: React.ChangeEvent<HTMLInputElement>) => {
    const value = e.target.value;
    setAmount(value);
    if (value && price) {
      const priceNum = parseFloat(price);
      const amountNum = parseFloat(value);
      if (!isNaN(priceNum) && !isNaN(amountNum) && priceNum > 0 && amountNum > 0) {
        setTotal((priceNum * amountNum).toFixed(2));
      } else {
        setTotal('');
      }
    } else {
      setTotal('');
    }
  };

  const handleSubmit = (e: React.FormEvent) => {
    e.preventDefault();
    // TODO: 주문 API 호출
    console.log('주문 제출:', { orderType, price, amount, total });
  };

  const setPercentage = (percent: number) => {
    // TODO: 실제 잔액 가져오기
    const balance = 1000; // 임시
    const calculatedAmount = ((balance * percent) / 100).toFixed(4);
    setAmount(calculatedAmount);
    if (price) {
      const priceNum = parseFloat(price);
      const amountNum = parseFloat(calculatedAmount);
      if (!isNaN(priceNum) && !isNaN(amountNum) && priceNum > 0 && amountNum > 0) {
        setTotal((priceNum * amountNum).toFixed(2));
      }
    }
  };

  return (
    <div className="bg-gray-800 rounded-lg border border-gray-700 p-5 h-full flex flex-col">
      <div className="flex mb-4">
        <button
          onClick={() => setOrderType('buy')}
          className={`flex-1 py-2.5 px-4 rounded-l font-semibold text-sm transition-colors ${
            orderType === 'buy'
              ? 'bg-blue-600 text-white'
              : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
          }`}
        >
          매수
        </button>
        <button
          onClick={() => setOrderType('sell')}
          className={`flex-1 py-2.5 px-4 rounded-r font-semibold text-sm transition-colors ${
            orderType === 'sell'
              ? 'bg-red-600 text-white'
              : 'bg-gray-700 text-gray-300 hover:bg-gray-600'
          }`}
        >
          매도
        </button>
      </div>

      <form onSubmit={handleSubmit} className="flex-1 flex flex-col space-y-4">
        <div>
          <label className="block text-sm text-gray-400 mb-1">가격 (USDT)</label>
          <input
            type="number"
            step="0.01"
            value={price}
            onChange={handlePriceChange}
            placeholder="0.00"
            className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
          />
        </div>

        <div>
          <label className="block text-sm text-gray-400 mb-1">수량 (SOL)</label>
          <input
            type="number"
            step="0.0001"
            value={amount}
            onChange={handleAmountChange}
            placeholder="0.0000"
            className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-sm text-white focus:outline-none focus:border-blue-500"
          />
          <div className="flex gap-2 mt-1.5">
            {[25, 50, 75, 100].map((percent) => (
              <button
                key={percent}
                type="button"
                onClick={() => setPercentage(percent)}
                className="flex-1 py-1 px-2 bg-gray-700 text-gray-300 text-xs rounded hover:bg-gray-600"
              >
                {percent}%
              </button>
            ))}
          </div>
        </div>

        <div>
          <label className="block text-sm text-gray-400 mb-1">총액 (USDT)</label>
          <input
            type="text"
            value={total}
            readOnly
            className="w-full bg-gray-900 border border-gray-600 rounded px-3 py-2 text-sm text-gray-400"
          />
        </div>

        <button
          type="submit"
          className={`w-full py-2.5 rounded font-semibold transition-colors mt-auto ${
            orderType === 'buy'
              ? 'bg-blue-600 hover:bg-blue-700 text-white'
              : 'bg-red-600 hover:bg-red-700 text-white'
          }`}
        >
          주문하기
        </button>
      </form>
    </div>
  );
}

