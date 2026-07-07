import React, { useState, useCallback } from 'react';
import { DataGrid, EmptyState, Button } from '@flint/react';
import type { FlintColumn } from '@flint/react';

interface Order {
  id: string;
  status: 'pending' | 'shipped' | 'delivered' | 'cancelled';
  customer: string;
  total: number;
  created_at: string;
}

const COLUMNS: FlintColumn[] = [
  { name: 'id',         type: 'uuid',    sortable: false, hidden: true },
  { name: 'customer',   type: 'text',    sortable: true  },
  { name: 'status',     type: 'text',    sortable: true  },
  { name: 'total',      type: 'number',  sortable: true, format: 'currency' },
  { name: 'created_at', type: 'date',    sortable: true  },
];

interface OrdersGridProps {
  orders: Order[];
  totalCount: number;
  isLoading?: boolean;
  onPageChange: (page: number) => void;
  onRowClick: (order: Order) => void;
}

export function OrdersGrid({
  orders,
  totalCount,
  isLoading = false,
  onPageChange,
  onRowClick,
}: OrdersGridProps) {
  const [sortState, setSortState] = useState<{ col: string; dir: 'asc' | 'desc' } | null>(null);

  const handleSort = useCallback((col: string, dir: 'asc' | 'desc') => {
    setSortState({ col, dir });
  }, []);

  return (
    <DataGrid
      columns={COLUMNS}
      data={orders}
      loading={isLoading}
      pagination={{
        pageSize: 25,
        totalRows: totalCount,
        onPageChange,
      }}
      sort={sortState ?? undefined}
      onSort={handleSort}
      onRowClick={onRowClick}
      emptyState={
        <EmptyState
          title="No orders found"
          description="Try adjusting your filters or search terms."
          action={
            <Button variant="outline" onClick={() => {}}>
              Clear Filters
            </Button>
          }
        />
      }
    />
  );
}
