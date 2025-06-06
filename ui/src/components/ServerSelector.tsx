import { useEffect } from 'react';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select';
import { Badge } from '@/components/ui/badge';
import { Button } from '@/components/ui/button';
import { RefreshCw, Server, AlertCircle, CheckCircle, Loader2 } from 'lucide-react';
import { useAppStore } from '@/lib/store';
import { apiClient } from '@/lib/api';

export function ServerSelector() {
  const {
    servers,
    selectedServerId,
    loading,
    error,
    setServers,
    setSelectedServerId,
    setLoading,
    setError,
  } = useAppStore();

  const loadServers = async () => {
    try {
      setLoading('servers', true);
      setError(null);
      const servers = await apiClient.listServers();
      setServers(servers);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load servers');
    } finally {
      setLoading('servers', false);
    }
  };

  useEffect(() => {
    loadServers();
  }, []);

  const getStatusIcon = (connected: boolean, error?: string) => {
    if (error) {
      return <AlertCircle className="w-4 h-4 text-red-500" />;
    } else if (connected) {
      return <CheckCircle className="w-4 h-4 text-green-500" />;
    } else {
      return <AlertCircle className="w-4 h-4 text-gray-400" />;
    }
  };

  const getStatusBadge = (connected: boolean, error?: string) => {
    if (error) {
      return <Badge variant="destructive">Error</Badge>;
    } else if (connected) {
      return <Badge variant="default" className="text-green-700 bg-green-100">Connected</Badge>;
    } else {
      return <Badge variant="secondary">Disconnected</Badge>;
    }
  };

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <div className="flex items-center space-x-2">
          <Server className="w-5 h-5 text-gray-600" />
          <h3 className="text-lg font-semibold">gRPC Servers</h3>
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={loadServers}
          disabled={loading.servers}
        >
          {loading.servers ? (
            <Loader2 className="w-4 h-4 animate-spin" />
          ) : (
            <RefreshCw className="w-4 h-4" />
          )}
        </Button>
      </div>

      {error && (
        <div className="p-3 bg-red-50 rounded-md border border-red-200">
          <div className="flex items-center space-x-2">
            <AlertCircle className="w-4 h-4 text-red-500" />
            <span className="text-sm text-red-700">{error}</span>
          </div>
        </div>
      )}

      <div className="space-y-3">
        <Select value={selectedServerId || ''} onValueChange={setSelectedServerId}>
          <SelectTrigger>
            <SelectValue placeholder="Select a gRPC server" />
          </SelectTrigger>
          <SelectContent>
            {servers.map((server) => (
              <SelectItem key={server.id} value={server.id}>
                <div className="flex items-center space-x-2 w-full">
                  {getStatusIcon(server.connected, server.error)}
                  <span className="flex-1">{server.name}</span>
                  <span className="text-xs text-gray-500">{server.endpoint}</span>
                </div>
              </SelectItem>
            ))}
          </SelectContent>
        </Select>

        {selectedServerId && (
          <div className="space-y-2">
            {servers
              .filter((s) => s.id === selectedServerId)
              .map((server) => (
                <div key={server.id} className="p-3 bg-gray-50 rounded-md">
                  <div className="flex justify-between items-center mb-2">
                    <h4 className="font-medium">{server.name}</h4>
                    {getStatusBadge(server.connected, server.error)}
                  </div>
                  <p className="mb-1 text-sm text-gray-600">
                    <strong>Endpoint:</strong> {server.endpoint}
                  </p>
                  {server.description && (
                    <p className="mb-1 text-sm text-gray-600">
                      <strong>Description:</strong> {server.description}
                    </p>
                  )}
                  {server.lastConnected && (
                    <p className="mb-1 text-sm text-gray-600">
                      <strong>Last Connected:</strong> {new Date(server.lastConnected).toLocaleString()}
                    </p>
                  )}
                  {server.error && (
                    <p className="text-sm text-red-600">
                      <strong>Error:</strong> {server.error}
                    </p>
                  )}
                </div>
              ))}
          </div>
        )}
      </div>
    </div>
  );
}
