import { Button } from './components/ui/button';
import { Moon, Sun, Settings, Zap } from 'lucide-react';
import { useAppStore } from '@/lib/store';
import { ServerSelector } from '@/components/ServerSelector';
import { ServiceList } from '@/components/ServiceList';
import { MethodList } from '@/components/MethodList';
import { RequestForm } from '@/components/RequestForm';
import { CallHistory } from '@/components/CallHistory';

function App() {
  const { theme, setTheme } = useAppStore();

  const toggleTheme = () => {
    setTheme(theme === 'light' ? 'dark' : 'light');
  };

  return (
    <div className={`min-h-screen ${theme === 'dark' ? 'dark' : ''}`}>
      <div className="bg-background text-foreground">
        {/* Header */}
        <header className="bg-white border-b dark:bg-gray-900">
          <div className="container px-4 py-4 mx-auto">
            <div className="flex justify-between items-center">
              <div className="flex items-center space-x-3">
                <div className="flex items-center space-x-2">
                  <Zap className="w-8 h-8 text-blue-600" />
                  <h1 className="text-2xl font-bold text-gray-900 dark:text-white">
                    gRPC Client
                  </h1>
                </div>
                <span className="text-sm text-gray-500 dark:text-gray-400">
                  Web Interface
                </span>
              </div>

              <div className="flex items-center space-x-3">
                <Button
                  variant="outline"
                  size="sm"
                  onClick={toggleTheme}
                  className="gap-2"
                >
                  {theme === 'light' ? (
                    <Moon className="w-4 h-4" />
                  ) : (
                    <Sun className="w-4 h-4" />
                  )}
                  {theme === 'light' ? 'Dark' : 'Light'}
                </Button>

                <Button variant="outline" size="sm" className="gap-2">
                  <Settings className="w-4 h-4" />
                  Settings
                </Button>
              </div>
            </div>
          </div>
        </header>

        {/* Main Content - Three Column Layout */}
        <main className="h-[calc(100vh-80px)] flex">
          {/* Left Sidebar - Selection */}
          <div className="overflow-y-auto w-80 bg-white border-r dark:bg-gray-900">
            <div className="p-4 space-y-6">
              {/* Server Selection */}
              <div className="space-y-4">
                <ServerSelector />
              </div>

              {/* Service Selection */}
              <div className="space-y-4">
                <ServiceList />
              </div>

              {/* Method Selection */}
              <div className="space-y-4">
                <MethodList />
              </div>
            </div>
          </div>

          {/* Middle Sidebar - Request Form */}
          <div className="overflow-y-auto w-96 bg-gray-50 border-r dark:bg-gray-800">
            <div className="p-4">
              <RequestForm />
            </div>
          </div>

          {/* Main Content - Call History */}
          <div className="overflow-y-auto flex-1 bg-white dark:bg-gray-900">
            <div className="p-4">
              <CallHistory />
            </div>
          </div>
        </main>

        {/* Footer */}
        <footer className="bg-gray-50 border-t dark:bg-gray-900">
          <div className="container px-4 py-2 mx-auto">
            <div className="flex justify-between items-center">
              <p className="text-xs text-gray-600 dark:text-gray-400">
                gRPC Client Web UI - Built with React & Rust
              </p>
              <div className="flex items-center space-x-4">
                <span className="text-xs text-gray-500">v0.1.0</span>
                <div className="flex items-center space-x-1">
                  <div className="w-2 h-2 bg-green-500 rounded-full"></div>
                  <span className="text-xs text-gray-500">Connected</span>
                </div>
              </div>
            </div>
          </div>
        </footer>
      </div>
    </div>
  );
}

export default App;
