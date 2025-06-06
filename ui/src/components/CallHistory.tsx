import { useState } from 'react';
import { Button } from '@/components/ui/button';
import { Badge } from '@/components/ui/badge';
import { Card, CardContent, CardHeader, CardTitle } from '@/components/ui/card';
import { Input } from '@/components/ui/input';
import {
  Clock,
  Server,
  Zap,
  Copy,
  CheckCircle,
  AlertCircle,
  Search,
  Trash2,
  ChevronDown,
  ChevronRight
} from 'lucide-react';
import { useAppStore } from '../lib/store';

export function CallHistory() {
  const { callHistory, clearCallHistory } = useAppStore();
  const [searchTerm, setSearchTerm] = useState('');
  const [expandedItems, setExpandedItems] = useState<Set<string>>(new Set());

  const filteredHistory = callHistory.filter((entry) => {
    if (!searchTerm) return true;
    const search = searchTerm.toLowerCase();
    return (
      entry.serviceName.toLowerCase().includes(search) ||
      entry.methodName.toLowerCase().includes(search) ||
      entry.serverId.toLowerCase().includes(search)
    );
  });

  const toggleExpanded = (id: string) => {
    const newExpanded = new Set(expandedItems);
    if (newExpanded.has(id)) {
      newExpanded.delete(id);
    } else {
      newExpanded.add(id);
    }
    setExpandedItems(newExpanded);
  };

  const copyToClipboard = (text: string) => {
    navigator.clipboard.writeText(text);
  };

  const formatDuration = (duration: number) => {
    if (duration < 1000) {
      return `${duration}ms`;
    }
    return `${(duration / 1000).toFixed(2)}s`;
  };

  const formatTimestamp = (timestamp: string) => {
    return new Date(timestamp).toLocaleString();
  };

  if (callHistory.length === 0) {
    return (
      <Card>
        <CardHeader>
          <CardTitle className="flex gap-2 items-center">
            <Clock className="w-5 h-5" />
            Call History
          </CardTitle>
        </CardHeader>
        <CardContent>
          <div className="p-8 text-center text-gray-500">
            <Clock className="mx-auto mb-3 w-12 h-12 text-gray-400" />
            <p className="text-sm">No calls made yet</p>
            <p className="mt-1 text-xs text-gray-400">
              Make a gRPC call to see the history here
            </p>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <div className="space-y-4">
      {/* Header with Search and Clear */}
      <Card>
        <CardHeader className="pb-3">
          <div className="flex justify-between items-center">
            <CardTitle className="flex gap-2 items-center">
              <Clock className="w-5 h-5" />
              Call History
              <Badge variant="outline" className="ml-2">
                {callHistory.length}
              </Badge>
            </CardTitle>
            <Button
              variant="outline"
              size="sm"
              onClick={clearCallHistory}
              className="gap-1"
              disabled={callHistory.length === 0}
            >
              <Trash2 className="w-3 h-3" />
              Clear
            </Button>
          </div>
        </CardHeader>
        <CardContent>
          <div className="relative">
            <Search className="absolute left-3 top-1/2 w-4 h-4 text-gray-400 transform -translate-y-1/2" />
            <Input
              placeholder="Search calls..."
              value={searchTerm}
              onChange={(e) => setSearchTerm(e.target.value)}
              className="pl-10"
            />
          </div>
        </CardContent>
      </Card>

      {/* Call History List */}
      <div className="space-y-3">
        {filteredHistory.map((entry) => {
          const isExpanded = expandedItems.has(entry.id);
          const isSuccess = entry.response.success;

          return (
            <Card key={entry.id} className={`transition-colors ${isSuccess ? 'border-green-200 bg-green-50/50' : 'border-red-200 bg-red-50/50'
              }`}>
              {/* Call Summary */}
              <CardHeader className="pb-3 cursor-pointer" onClick={() => toggleExpanded(entry.id)}>
                <div className="flex justify-between items-center">
                  <div className="flex gap-3 items-center">
                    <div className="flex gap-1 items-center">
                      {isExpanded ? (
                        <ChevronDown className="w-4 h-4" />
                      ) : (
                        <ChevronRight className="w-4 h-4" />
                      )}
                      {isSuccess ? (
                        <CheckCircle className="w-4 h-4 text-green-500" />
                      ) : (
                        <AlertCircle className="w-4 h-4 text-red-500" />
                      )}
                    </div>

                    <div className="flex-1 min-w-0">
                      <div className="flex gap-2 items-center">
                        <span className="text-sm font-medium truncate">
                          {entry.serviceName}.{entry.methodName}
                        </span>
                        <Badge variant="outline" className="text-xs">
                          {entry.serverId}
                        </Badge>
                      </div>
                      <div className="mt-1 text-xs text-gray-500">
                        {formatTimestamp(entry.timestamp)} • {formatDuration(entry.duration)}
                      </div>
                    </div>
                  </div>

                  <div className="flex gap-2 items-center">
                    <Badge
                      variant={isSuccess ? "default" : "destructive"}
                      className="text-xs"
                    >
                      {isSuccess ? 'Success' : 'Error'}
                    </Badge>
                    {entry.response.response && (
                      <Badge variant="secondary" className="text-xs">
                        {entry.response.response.length} response(s)
                      </Badge>
                    )}
                  </div>
                </div>
              </CardHeader>

              {/* Expanded Details */}
              {isExpanded && (
                <CardContent className="space-y-4 bg-white border-t">
                  {/* Request Section */}
                  <div className="space-y-2">
                    <div className="flex justify-between items-center">
                      <h4 className="flex gap-1 items-center text-sm font-medium">
                        <Zap className="w-3 h-3" />
                        Request
                      </h4>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => copyToClipboard(JSON.stringify(entry.request, null, 2))}
                        className="gap-1 px-2 h-6"
                      >
                        <Copy className="w-3 h-3" />
                        Copy
                      </Button>
                    </div>
                    <pre className="overflow-auto p-3 max-h-40 text-xs bg-gray-50 rounded border">
                      {JSON.stringify(entry.request, null, 2)}
                    </pre>
                  </div>

                  {/* Headers Section (if present) */}
                  {entry.headers && Object.keys(entry.headers).length > 0 && (
                    <div className="space-y-2">
                      <div className="flex justify-between items-center">
                        <h4 className="flex gap-1 items-center text-sm font-medium">
                          <Server className="w-3 h-3" />
                          Headers
                        </h4>
                        <Button
                          variant="ghost"
                          size="sm"
                          onClick={() => copyToClipboard(JSON.stringify(entry.headers, null, 2))}
                          className="gap-1 px-2 h-6"
                        >
                          <Copy className="w-3 h-3" />
                          Copy
                        </Button>
                      </div>
                      <pre className="overflow-auto p-3 max-h-20 text-xs bg-gray-50 rounded border">
                        {JSON.stringify(entry.headers, null, 2)}
                      </pre>
                    </div>
                  )}

                  {/* Response Section */}
                  <div className="space-y-2">
                    <div className="flex justify-between items-center">
                      <h4 className="flex gap-1 items-center text-sm font-medium">
                        {isSuccess ? (
                          <CheckCircle className="w-3 h-3 text-green-500" />
                        ) : (
                          <AlertCircle className="w-3 h-3 text-red-500" />
                        )}
                        Response
                      </h4>
                      <Button
                        variant="ghost"
                        size="sm"
                        onClick={() => copyToClipboard(JSON.stringify(entry.response, null, 2))}
                        className="gap-1 px-2 h-6"
                      >
                        <Copy className="w-3 h-3" />
                        Copy
                      </Button>
                    </div>
                    <pre className={`text-xs p-3 rounded border overflow-auto max-h-60 ${isSuccess ? 'bg-green-50 border-green-200' : 'bg-red-50 border-red-200'
                      }`}>
                      {JSON.stringify(entry.response, null, 2)}
                    </pre>
                  </div>

                  {/* Call Details */}
                  <div className="flex justify-between items-center pt-2 text-xs text-gray-500 border-t">
                    <div className="flex gap-4 items-center">
                      <span>Duration: {formatDuration(entry.duration)}</span>
                      <span>ID: {entry.id}</span>
                    </div>
                    <span>{formatTimestamp(entry.timestamp)}</span>
                  </div>
                </CardContent>
              )}
            </Card>
          );
        })}
      </div>

      {filteredHistory.length === 0 && searchTerm && (
        <Card>
          <CardContent className="p-8 text-center text-gray-500">
            <Search className="mx-auto mb-3 w-12 h-12 text-gray-400" />
            <p className="text-sm">No calls found matching "{searchTerm}"</p>
          </CardContent>
        </Card>
      )}
    </div>
  );
}
