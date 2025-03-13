import { useState, useEffect } from 'react';

interface VideoInfo {
  title: string;
  uploader: string;
  duration: number;
  views: number;
  likes: number;
  uploadDate: string;
}

interface DownloadFormProps {
  isPro: boolean;
  onDownloadStart: () => void;
}

const DownloadForm: React.FC<DownloadFormProps> = ({ isPro, onDownloadStart }) => {
  const [url, setUrl] = useState('');
  const [quality, setQuality] = useState('720');
  const [format, setFormat] = useState('mp4');
  const [startTime, setStartTime] = useState('');
  const [endTime, setEndTime] = useState('');
  const [usePlaylist, setUsePlaylist] = useState(false);
  const [downloadSubtitles, setDownloadSubtitles] = useState(false);
  const [outputDir, setOutputDir] = useState('');
  const [showAdvanced, setShowAdvanced] = useState(false);
  const [isLoading, setIsLoading] = useState(false);
  const [error, setError] = useState('');
  const [videoInfo, setVideoInfo] = useState<VideoInfo | null>(null);

  // Reset error when URL changes
  useEffect(() => {
    setError('');
  }, [url]);

  const fetchVideoInfo = async (): Promise<void> => {
    if (!url || url.length < 5) return;
    
    setIsLoading(true);
    try {
      // In a real app, this would fetch video info from a backend API
      // For demo purposes, we'll simulate a successful API call
      setTimeout(() => {
        const mockVideoInfo: VideoInfo = {
          title: "Sample Video Title",
          uploader: "Sample Channel",
          duration: 325, // 5:25 in seconds
          views: 12500,
          likes: 1050,
          uploadDate: "2023-10-15"
        };
        setVideoInfo(mockVideoInfo);
        setError('');
        setIsLoading(false);
      }, 1000);
    } catch (err) {
      setError(`Failed to fetch video info: ${err instanceof Error ? err.message : 'Unknown error'}`);
      setVideoInfo(null);
      setIsLoading(false);
    }
  };

  const selectOutputDirectory = async (): Promise<void> => {
    try {
      // In a real app, this would use a file picker API
      // For demo purposes, we'll simulate a successful directory selection
      setTimeout(() => {
        setOutputDir('/Users/username/Downloads/Videos');
      }, 500);
    } catch (err) {
      setError(`Failed to select directory: ${err instanceof Error ? err.message : 'Unknown error'}`);
    }
  };

  const validateTimeFormat = (value: string): boolean => {
    if (!value) return true;
    // Format HH:MM:SS
    const timeRegex = /^([0-9][0-9]):([0-5][0-9]):([0-5][0-9])$/;
    return timeRegex.test(value);
  };

  const handleSubmit = async (e: React.FormEvent): Promise<void> => {
    e.preventDefault();
    
    if (!url) {
      setError('Please enter a URL');
      return;
    }
    
    if (startTime && !validateTimeFormat(startTime)) {
      setError('Invalid start time format. Use HH:MM:SS');
      return;
    }
    
    if (endTime && !validateTimeFormat(endTime)) {
      setError('Invalid end time format. Use HH:MM:SS');
      return;
    }
    
    setIsLoading(true);
    setError('');
    
    try {
      // Notify parent component about download starting
      onDownloadStart();
      
      // In a real app, this would call a backend API to start the download
      // For demo purposes, we'll simulate a successful download start
      setTimeout(() => {
        setIsLoading(false);
      }, 1500);
    } catch (err) {
      setError(`Download failed: ${err instanceof Error ? err.message : 'Unknown error'}`);
      setIsLoading(false);
    }
  };

  return (
    <div className="bg-white dark:bg-gray-800 rounded-lg shadow-md p-6">
      <form onSubmit={handleSubmit} className="space-y-4">
        {/* Video URL Input */}
        <div className="space-y-2">
          <div className="flex items-center justify-between">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Video URL
            </label>
            {isLoading && (
              <span className="text-xs text-blue-500">Loading...</span>
            )}
          </div>
          <div className="flex space-x-2">
            <input
              type="text"
              value={url}
              onChange={(e) => setUrl(e.target.value)}
              onBlur={fetchVideoInfo}
              placeholder="https://www.youtube.com/watch?v=..."
              disabled={isLoading}
              className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
              required
            />
            <button
              type="button"
              onClick={fetchVideoInfo}
              disabled={isLoading}
              className="px-3 py-2 bg-blue-500 text-white rounded-md text-sm hover:bg-blue-600 transition-colors disabled:bg-blue-300"
            >
              Fetch Info
            </button>
          </div>
        </div>

        {/* Video Info Preview (if available) */}
        {videoInfo && (
          <div className="p-4 bg-gray-100 dark:bg-gray-700 rounded-md">
            <h3 className="font-medium text-sm mb-2">{videoInfo.title}</h3>
            <div className="flex space-x-4 text-xs text-gray-600 dark:text-gray-300">
              <span>Duration: {Math.floor(videoInfo.duration / 60)}:{(videoInfo.duration % 60).toString().padStart(2, '0')}</span>
              {videoInfo.uploader && <span>By: {videoInfo.uploader}</span>}
            </div>
          </div>
        )}

        {/* Error Message */}
        {error && (
          <div className="p-3 bg-red-100 text-red-800 dark:bg-red-900 dark:text-red-200 rounded-md text-sm">
            {error}
          </div>
        )}

        {/* Format and Quality Section */}
        <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
          <div className="space-y-2">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Quality
            </label>
            <select
              value={quality}
              onChange={(e) => setQuality(e.target.value)}
              disabled={isLoading}
              className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            >
              <option value="480">480p</option>
              <option value="720">720p</option>
              {isPro && (
                <>
                  <option value="1080">1080p</option>
                  <option value="2160">4K</option>
                </>
              )}
            </select>
            {!isPro && (
              <p className="text-xs text-amber-600 dark:text-amber-400">
                Pro version required for 1080p/4K
              </p>
            )}
          </div>

          <div className="space-y-2">
            <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
              Format
            </label>
            <select
              value={format}
              onChange={(e) => setFormat(e.target.value)}
              disabled={isLoading}
              className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
            >
              <option value="mp4">MP4 Video</option>
              <option value="mp3">MP3 Audio</option>
              {isPro && (
                <>
                  <option value="webm">WebM</option>
                  <option value="flac">FLAC Audio</option>
                </>
              )}
            </select>
          </div>
        </div>

        {/* Toggle Advanced Options */}
        <div className="pt-2">
          <button 
            type="button" 
            onClick={() => setShowAdvanced(!showAdvanced)}
            disabled={isLoading}
            className="text-sm text-blue-600 hover:text-blue-800 dark:text-blue-400 dark:hover:text-blue-300"
          >
            {showAdvanced ? 'Hide Advanced Options' : 'Show Advanced Options'}
          </button>
        </div>

        {/* Advanced Options Section */}
        {showAdvanced && (
          <div className="space-y-4 pt-2 border-t dark:border-gray-700">
            {/* Time Range */}
            <div className="grid grid-cols-1 md:grid-cols-2 gap-4">
              <div className="space-y-2">
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                  Start Time (HH:MM:SS)
                </label>
                <input
                  type="text"
                  value={startTime}
                  onChange={(e) => setStartTime(e.target.value)}
                  placeholder="00:00:00"
                  disabled={isLoading}
                  className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
                {startTime && !validateTimeFormat(startTime) && (
                  <p className="text-xs text-red-600">Format must be HH:MM:SS</p>
                )}
              </div>

              <div className="space-y-2">
                <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                  End Time (HH:MM:SS)
                </label>
                <input
                  type="text"
                  value={endTime}
                  onChange={(e) => setEndTime(e.target.value)}
                  placeholder="00:00:00"
                  disabled={isLoading}
                  className="w-full p-2 border rounded-md text-sm dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
                {endTime && !validateTimeFormat(endTime) && (
                  <p className="text-xs text-red-600">Format must be HH:MM:SS</p>
                )}
              </div>
            </div>

            {/* Checkboxes */}
            <div className="flex flex-col sm:flex-row space-y-2 sm:space-y-0 sm:space-x-6">
              <label className="inline-flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={usePlaylist}
                  onChange={(e) => setUsePlaylist(e.target.checked)}
                  disabled={isLoading}
                  className="rounded text-blue-600 dark:bg-gray-700"
                />
                <span className="text-sm text-gray-700 dark:text-gray-300">Download entire playlist</span>
              </label>

              <label className="inline-flex items-center space-x-2">
                <input
                  type="checkbox"
                  checked={downloadSubtitles}
                  onChange={(e) => setDownloadSubtitles(e.target.checked)}
                  disabled={isLoading}
                  className="rounded text-blue-600 dark:bg-gray-700"
                />
                <span className="text-sm text-gray-700 dark:text-gray-300">Download subtitles</span>
              </label>
            </div>

            {/* Output Directory */}
            <div className="space-y-2">
              <label className="block text-sm font-medium text-gray-700 dark:text-gray-300">
                Output Directory
              </label>
              <div className="flex space-x-2">
                <input
                  type="text"
                  value={outputDir}
                  readOnly
                  placeholder="Default directory"
                  disabled={isLoading}
                  className="w-full p-2 border rounded-md text-sm bg-gray-50 dark:bg-gray-700 dark:border-gray-600 dark:text-white"
                />
                <button
                  type="button"
                  onClick={selectOutputDirectory}
                  disabled={isLoading}
                  className="px-3 py-2 bg-gray-500 text-white rounded-md text-sm hover:bg-gray-600 transition-colors disabled:bg-gray-400"
                >
                  Browse
                </button>
              </div>
            </div>
          </div>
        )}

        {/* Submit Button */}
        <div className="pt-2">
        <button
  type="submit"
  disabled={!!(isLoading || !url || (startTime && !validateTimeFormat(startTime)) || (endTime && !validateTimeFormat(endTime)))}
  className="w-full py-2 px-4 bg-blue-600 hover:bg-blue-700 text-white font-medium rounded-md shadow-sm disabled:bg-blue-300 transition-colors"
>
  {isLoading ? 'Processing...' : 'Download'}
</button>
        </div>
      </form>
    </div>
  );
};

export default DownloadForm;