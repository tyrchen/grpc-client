import { useEffect } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { RefreshCw, Database, Loader2, AlertCircle, ChevronRight } from 'lucide-react';
import { useAppStore, useSelectedServer } from '@/lib/store';
import { apiClient } from '@/lib/api';

export function ServiceList() {
  const {
    selectedServerId,
    services,
    selectedServiceName,
    loading,
    error,
    setServices,
    setSelectedServiceName,
    setLoading,
    setError,
  } = useAppStore();

  const selectedServer = useSelectedServer();

  const loadServices = async () => {
    if (!selectedServerId) return;

    try {
      setLoading('services', true);
      setError(null);
      const response = await apiClient.listServices(selectedServerId);

      // Convert the service names to ServiceInfo objects, filtering out reflection services
      const serviceInfos = response.services
        .filter((serviceName: string) => !serviceName.includes('reflection'))
        .map((serviceName: string) => ({
          name: serviceName,
          description: undefined,
          methods: [],
        }));

      setServices(serviceInfos);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Failed to load services');
      setServices([]);
    } finally {
      setLoading('services', false);
    }
  };

  useEffect(() => {
    if (selectedServerId) {
      loadServices();
    } else {
      setServices([]);
      setSelectedServiceName(null);
    }
  }, [selectedServerId]);

  const handleServiceSelect = (serviceName: string) => {
    setSelectedServiceName(serviceName === selectedServiceName ? null : serviceName);
  };

  if (!selectedServerId) {
    return (
      <div className="space-y-4">
        <div className="flex items-center space-x-2">
          <Database className="w-5 h-5 text-gray-400" />
          <h3 className="text-lg font-semibold text-gray-400">Services</h3>
        </div>
        <div className="p-4 text-center text-gray-500 bg-gray-50 rounded-lg">
          Select a server to view services
        </div>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      <div className="flex justify-between items-center">
        <div className="flex items-center space-x-2">
          <Database className="w-5 h-5 text-gray-600" />
          <h3 className="text-lg font-semibold">Services</h3>
          {selectedServer && (
            <Badge variant="outline" className="text-xs">
              {selectedServer.name}
            </Badge>
          )}
        </div>
        <Button
          variant="outline"
          size="sm"
          onClick={loadServices}
          disabled={loading.services}
        >
          {loading.services ? (
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

      {loading.services ? (
        <div className="space-y-2">
          {[1, 2, 3].map((i) => (
            <div key={i} className="p-3 bg-gray-100 rounded-md animate-pulse">
              <div className="w-3/4 h-4 bg-gray-300 rounded"></div>
            </div>
          ))}
        </div>
      ) : services.length > 0 ? (
        <div className="space-y-2">
          {services.map((service) => (
            <div
              key={service.name}
              className={`p-3 border rounded-md cursor-pointer transition-colors ${selectedServiceName === service.name
                ? 'bg-blue-50 border-blue-200'
                : 'bg-white border-gray-200 hover:bg-gray-50'
                }`}
              onClick={() => handleServiceSelect(service.name)}
            >
              <div className="flex justify-between items-center">
                <div className="flex-1 min-w-0">
                  <div className="flex items-center space-x-2">
                    <span className="font-medium text-gray-900 truncate">
                      {service.name}
                    </span>
                    {service.name.includes('reflection') && (
                      <Badge variant="secondary" className="text-xs">
                        Reflection
                      </Badge>
                    )}
                  </div>
                  {service.description && (
                    <p className="mt-1 text-sm text-gray-600">
                      {service.description}
                    </p>
                  )}
                </div>
                <ChevronRight
                  className={`w-4 h-4 text-gray-400 transition-transform ${selectedServiceName === service.name ? 'rotate-90' : ''
                    }`}
                />
              </div>
            </div>
          ))}
        </div>
      ) : (
        <div className="p-4 text-center text-gray-500 bg-gray-50 rounded-lg">
          No services found
        </div>
      )}

      {selectedServiceName && (
        <div className="p-3 bg-blue-50 rounded-md border border-blue-200">
          <p className="text-sm text-blue-700">
            <strong>Selected:</strong> {selectedServiceName}
          </p>
          <p className="mt-1 text-xs text-blue-600">
            Methods will be loaded automatically
          </p>
        </div>
      )}
    </div>
  );
}
