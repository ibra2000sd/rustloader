import React from 'react';
import './ProgressBar.css';

interface ProgressBarProps {
  progress: number;
  label: string;
}

const ProgressBar: React.FC<ProgressBarProps> = ({ progress, label }) => {
  // Ensure progress is between 0 and 100
  const normalizedProgress = Math.min(100, Math.max(0, progress));
  
  // Determine color based on progress
  const getProgressColor = () => {
    if (progress < 30) return '#3498db'; // Blue for early progress
    if (progress < 70) return '#2ecc71'; // Green for middle progress
    return '#f39c12'; // Orange for near completion
  };

  return (
    <div className="progress-container">
      <div className="progress-label">
        <span>{label}</span>
        <span className="progress-percent">{normalizedProgress}%</span>
      </div>
      <div className="progress-track">
        <div 
          className="progress-bar"
          style={{ 
            width: `${normalizedProgress}%`,
            backgroundColor: getProgressColor()
          }}
        ></div>
      </div>
      <div className="progress-status">
        {progress < 100 ? 'Downloading...' : 'Finalizing...'}
      </div>
    </div>
  );
};

export default ProgressBar;