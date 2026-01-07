# Rustloader v0.9.0 Feature Specifications

## 1. Format Selection UI

### User Story
As a user, I want to choose the video quality and format before downloading,
so I can balance file size with quality based on my needs.

### Requirements
- [ ] Display available formats after URL extraction
- [ ] Show: Resolution, Format, File Size (estimated), Codec
- [ ] Allow selection of video-only, audio-only, or combined
- [ ] Remember last selected quality preference
- [ ] Quick presets: "Best", "1080p", "720p", "Audio Only"

### UI Mockup
```
┌─────────────────────────────────────────────┐
│ Select Format                          [X]  │
├─────────────────────────────────────────────┤
│ Video Title Here                            │
│                                             │
│ Quick Select: [Best] [1080p] [720p] [Audio] │
│                                             │
│ ┌─────────────────────────────────────────┐ │
│ │ ○ 1080p MP4  - 150 MB - h264/aac       │ │
│ │ ● 720p MP4   - 80 MB  - h264/aac       │ │
│ │ ○ 480p MP4   - 40 MB  - h264/aac       │ │
│ │ ○ Audio MP3  - 5 MB   - mp3            │ │
│ │ ○ Audio M4A  - 8 MB   - aac            │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│              [Cancel]  [Download]           │
└─────────────────────────────────────────────┘
```

### Technical Implementation
1. Modify `VideoInfo` to include all available formats
2. Create `FormatSelector` component in GUI
3. Add format preference to Settings
4. Pass selected format ID to download engine

---

## 2. Playlist Support

### User Story
As a user, I want to download entire YouTube playlists,
so I don't have to paste each URL individually.

### Requirements
- [ ] Detect playlist URLs automatically
- [ ] Show playlist info: Title, Video Count, Total Duration
- [ ] Allow selecting specific videos from playlist
- [ ] Select All / Deselect All buttons
- [ ] Show download progress for each video
- [ ] Handle playlist pagination (>100 videos)

### UI Mockup
```
┌─────────────────────────────────────────────┐
│ Playlist: "My Favorite Songs" (25 videos)   │
├─────────────────────────────────────────────┤
│ [Select All] [Deselect All]    Format: 720p │
│                                             │
│ ☑ 1. Song Title One          3:45    50 MB │
│ ☑ 2. Song Title Two          4:12    55 MB │
│ ☐ 3. Song Title Three        5:01    65 MB │
│ ☑ 4. Song Title Four         3:30    45 MB │
│ ...                                         │
│                                             │
│ Selected: 20/25    Total: 1.2 GB            │
│              [Cancel]  [Download All]       │
└─────────────────────────────────────────────┘
```

### Technical Implementation
1. Add playlist detection in URL parser
2. Create `PlaylistInfo` model
3. Implement `PlaylistView` component
4. Modify queue manager for batch operations
5. Add playlist progress tracking

---

## 3. Batch URL Input

### User Story
As a user, I want to paste multiple URLs at once,
so I can queue many downloads quickly.

### Requirements
- [ ] Multi-line text input for URLs
- [ ] Parse and validate each URL
- [ ] Show validation status per URL
- [ ] Add all valid URLs to queue
- [ ] Option to extract info first or download directly

### UI Mockup
```
┌─────────────────────────────────────────────┐
│ Batch Download                         [X]  │
├─────────────────────────────────────────────┤
│ Paste URLs (one per line):                  │
│ ┌─────────────────────────────────────────┐ │
│ │ https://youtube.com/watch?v=abc123  ✅  │ │
│ │ https://vimeo.com/456789            ✅  │ │
│ │ invalid-url                         ❌  │ │
│ │ https://youtube.com/watch?v=xyz789  ✅  │ │
│ │                                         │ │
│ └─────────────────────────────────────────┘ │
│                                             │
│ Valid: 3/4    Quality: [720p ▼]             │
│                                             │
│        [Cancel]  [Add to Queue]             │
└─────────────────────────────────────────────┘
```

---

## Implementation Order

1. **Week 1-2**: Format Selection UI
   - Backend: Extend VideoInfo model
   - Frontend: Create FormatSelector component
   - Integration: Wire up to download engine

2. **Week 3-4**: Batch URL Input
   - Backend: URL parser improvements
   - Frontend: Multi-line input component
   - Integration: Queue batch operations

3. **Week 5-6**: Playlist Support
   - Backend: Playlist extraction via yt-dlp
   - Frontend: PlaylistView component
   - Integration: Batch download management

4. **Week 7**: Testing & Polish
   - Integration tests for new features
   - UI/UX refinements
   - Documentation updates
