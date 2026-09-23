export const resources = {
  en: {
    translation: {
      app: {
        name: "Just Convert",
        localWorkspace: "Local workspace",
        localOnly: "Private · Local only",
      },
      navigation: {
        primary: "Primary navigation",
        files: "Files",
        convert: "Convert",
        jobs: "Jobs",
        settings: "Settings",
      },
      preferences: {
        label: "Display preferences",
        language: "Language",
        theme: "Theme",
        system: "System",
        light: "Light",
        dark: "Dark",
      },
      settings: {
        eyebrow: "Application settings",
        title: "Settings",
        description:
          "Manage local preferences and optional conversion engines.",
        dependencies: {
          title: "Dependencies",
          description:
            "Just Convert checks manual paths first, then searches your system automatically.",
          ready: "Ready",
          notReady: "Setup required",
          autoPath: "Automatically detected",
          manualPath: "Manual path",
          notFound: "Not found automatically",
          browse: "Browse…",
          reset: "Reset to automatic detection",
          searchAgain: "Search again",
          searching: "Searching…",
          invalidPath:
            "The saved executable no longer exists or is invalid. Automatic fallback: {{fallback}}",
          windowsAppsAlias:
            "The saved path is a Microsoft Store alias, not a Python installation. Choose a Python executable outside WindowsApps.",
          versionCheckFailed:
            "Python 3.10+ could not be confirmed. The manual override was saved and will still be used.",
          noFallback: "not found",
          packagesMissing:
            "Python was found, but PyMuPDF and pdf2docx are not installed for this interpreter.",
          unavailable: "The dependency is currently unavailable.",
        },
      },
      dashboard: {
        privateEyebrow: "Private by design",
        title: "Your files, your machine.",
        description:
          "Browse, manage, and convert files without uploading them. Every operation stays on this computer.",
        browse: "Browse files",
      },
      converters: {
        eyebrow: "Capabilities",
        title: "Conversion tools",
        imagesToPdf: {
          title: "Images to PDF",
          description:
            "Combine JPG, PNG, WebP, and TIFF images into one local PDF.",
        },
        pdfTools: {
          title: "Organize PDF",
          description: "Merge, split, reorder, and rotate PDF pages with qpdf.",
        },
        pdfToImages: {
          title: "PDF to images",
          description: "Render PDF pages to high-quality PNG or JPEG files.",
        },
        audio: {
          title: "Convert audio",
          description:
            "Convert between MP3, WAV, FLAC, AAC, M4A, and OGG with FFmpeg.",
        },
        officeToPdf: {
          title: "Office to PDF",
          description:
            "Convert DOCX, XLSX, and PPTX files using local LibreOffice.",
        },
        pdfToDocx: {
          title: "PDF to Word",
          description:
            "Best-effort DOCX conversion for PDFs containing selectable text.",
        },
      },
      status: {
        ready: "Ready",
        external_required: "Setup required",
        planned: "Planned",
        bundled: "Bundled",
        external: "External",
      },
      batchMode: {
        label: "Source selection mode",
        file: "Single file",
        folder: "Folder batch",
      },
      audio: {
        back: "All conversion tools",
        title: "Convert audio",
        description:
          "Convert one audio file or a full folder with the bundled FFmpeg engine.",
        mode: {
          label: "Audio source mode",
          file: "Single file",
          folder: "Folder batch",
        },
        input: {
          label: "Source file",
          help: "MP3, WAV, FLAC, AAC, M4A, or OGG",
          choose: "Choose audio",
          change: "Change",
        },
        format: {
          label: "Output format",
          help: "MP3, WAV, FLAC, AAC, M4A, or OGG",
        },
        output: {
          label: "Save location",
          help: "Existing files are never overwritten.",
          choose: "Choose output",
        },
        folderInput: {
          label: "Source folder",
          help: "Supported audio files in this folder are included; subfolders are skipped.",
          choose: "Choose source folder",
        },
        folderOutput: {
          label: "Output folder",
          help: "Name conflicts receive -2, -3, and so on. Originals remain unchanged.",
          choose: "Choose output folder",
        },
        localNotice: "Runs locally · Originals remain unchanged",
        convert: "Add to queue",
        adding: "Adding…",
        queued: "Conversion added to the local queue.",
        batchQueued: "{{count}} audio files added to the local queue.",
        viewJobs: "View jobs",
      },
      images: {
        back: "All conversion tools",
        engine: "Native",
        title: "Images to PDF",
        description:
          "Combine local images into one PDF in the order you choose.",
        order: "Image order",
        orderHelp: "Each image becomes one page, from top to bottom.",
        add: "Add images",
        empty: "Add one or more JPG, PNG, WebP, or TIFF images.",
        moveUp: "Move {{name}} up",
        moveDown: "Move {{name}} down",
        remove: "Remove {{name}}",
        pageLayout: "Automatic A4 layout",
        pageLayoutHelp:
          "Portrait or landscape is selected per image. Aspect ratio is preserved with a small margin.",
        output: "Output PDF",
        outputHelp: "Existing files are never overwritten.",
        chooseOutput: "Choose output",
        localNotice: "Runs locally · Originals remain unchanged",
        addToQueue: "Add to queue",
        adding: "Adding…",
        queued: "Images-to-PDF conversion added to the local queue.",
        folderInput: "Source folder",
        folderInputHelp:
          "All supported images at the top level are included in filename order; subfolders are skipped.",
        changeFolder: "Change folder",
        chooseSourceFolder: "Choose source folder",
        folderMode: "PDF output layout",
        folderModeHelp: "Choose how the folder images should become PDFs.",
        combine: "Combine all images into one PDF",
        combineHelp: "Creates one multi-page PDF in filename order.",
        separate: "One PDF per image",
        separateHelp: "Creates a separate one-page PDF for every image.",
        folderOutput: "Output folder",
        folderOutputHelpCombine:
          "Creates one combined PDF and resolves name conflicts with -2, -3, and so on.",
        folderOutputHelpSeparate:
          "Uses each image's base filename and resolves conflicts with -2, -3, and so on.",
        chooseOutputFolder: "Choose output folder",
        batchQueuedCombine: "Folder images added as one combined PDF job.",
        batchQueuedSeparate: "{{count}} separate image-to-PDF jobs added.",
        viewJobs: "View jobs",
      },
      pdfImages: {
        back: "All conversion tools",
        title: "PDF to images",
        description:
          "Render every PDF page to a separate local PNG or JPEG image.",
        input: "Source PDF",
        inputHelp: "Choose one PDF to inspect its page count.",
        pageCount: "{{count}} pages",
        choosePdf: "Choose PDF",
        format: "Image format",
        formatHelp: "PNG is lossless; JPEG creates smaller files.",
        resolution: "Resolution",
        resolutionHelp: "Higher resolution uses more memory and disk space.",
        standard: "Standard · 150 DPI",
        high: "High · 300 DPI",
        output: "Output folder",
        outputHelp:
          "Names include format and quality; collisions are renamed with -2, -3, and so on.",
        chooseFolder: "Choose folder",
        localNotice: "Runs locally · One image per page",
        addToQueue: "Add to queue",
        adding: "Adding…",
        queued: "PDF-to-images conversion added to the local queue.",
        folderInput: "Source folder",
        folderInputHelp:
          "Every top-level PDF is included; subfolders and unsupported files are skipped.",
        chooseSourceFolder: "Choose source folder",
        folderOutputHelp:
          "Each PDF gets a collision-safe output subfolder named after the source file.",
        batchQueued: "{{count}} PDF files added to the local queue.",
        viewJobs: "View jobs",
      },
      office: {
        back: "All conversion tools",
        title: "Office to PDF",
        description:
          "Convert DOCX, XLSX, or PPTX files with your local LibreOffice installation.",
        input: "Office document",
        inputHelp: "DOCX, XLSX, or PPTX",
        chooseFile: "Choose document",
        output: "Output PDF",
        outputHelp: "Existing files are never overwritten.",
        chooseOutput: "Choose output",
        localNotice: "Runs locally · Uses an isolated LibreOffice profile",
        addToQueue: "Add to queue",
        adding: "Adding…",
        queued: "Office-to-PDF conversion added to the local queue.",
        folderInput: "Source folder",
        folderInputHelp:
          "Top-level DOCX, XLSX, and PPTX files are included; other files and subfolders are skipped.",
        chooseSourceFolder: "Choose source folder",
        folderOutputHelp:
          "One PDF is created per document; name conflicts receive -2, -3, and so on.",
        chooseOutputFolder: "Choose output folder",
        batchQueued: "{{count}} Office documents added to the local queue.",
        viewJobs: "View jobs",
        missing: {
          title: "LibreOffice is required",
          description:
            "Install LibreOffice, then restart Just Convert so it can be detected.",
          download: "Download LibreOffice",
          setPath: "Set path manually",
        },
      },
      pdfDocx: {
        back: "All conversion tools",
        title: "PDF to Word",
        description:
          "Convert a PDF containing selectable text to an editable DOCX using your local Python installation.",
        input: "Source PDF",
        inputHelp: "Digitally generated PDFs with selectable text only",
        chooseFile: "Choose PDF",
        output: "Output Word document",
        outputHelp: "Existing files are never overwritten.",
        chooseOutput: "Choose output",
        localNotice: "Runs locally · Originals remain unchanged",
        addToQueue: "Add to queue",
        adding: "Adding…",
        queued: "PDF-to-Word conversion added to the local queue.",
        folderInput: "Source folder",
        folderInputHelp:
          "Top-level PDFs are queued; unsupported files and subfolders are skipped. Scanned PDFs appear as failed jobs with a reason.",
        chooseSourceFolder: "Choose source folder",
        folderOutputHelp:
          "One DOCX is created per PDF; name conflicts receive -2, -3, and so on.",
        chooseOutputFolder: "Choose output folder",
        batchQueued: "{{count}} PDF files added to the local queue.",
        viewJobs: "View jobs",
        quality: {
          title: "Best-effort layout conversion",
          description:
            "Text, tables, and simple layouts usually convert reasonably. Complex columns, graphics, and precise formatting may differ. Image-only scanned PDFs are rejected; OCR will be added later.",
        },
        missing: {
          title: "Python conversion dependencies are required",
          description:
            "Install Python 3.10 or newer, then install PyMuPDF and pdf2docx with the command below. Restart Just Convert after installation.",
          python: "Download Python",
          copyCommand: "Copy install command",
          copy: "Copy",
          copied: "Copied",
          copyFailed: "Copy failed",
          setPath: "Set path manually",
        },
      },
      pdf: {
        back: "All conversion tools",
        title: "Organize PDF",
        description:
          "Merge, split, reorder, or rotate PDF pages locally with qpdf.",
        modeLabel: "PDF operation",
        modes: {
          merge: "Merge",
          split: "Split",
          reorder: "Reorder",
          rotate: "Rotate",
        },
        choosePdfs: "Add PDFs",
        choosePdf: "Choose PDF",
        chooseFolder: "Choose folder",
        chooseOutput: "Choose output",
        source: "Source PDF",
        sourceHelp: "Choose one PDF to inspect its page count.",
        pageCount: "{{count}} pages",
        output: "Output PDF",
        outputHelp: "Existing files are never overwritten.",
        moveUp: "Move {{name}} up",
        moveDown: "Move {{name}} down",
        remove: "Remove {{name}}",
        merge: {
          files: "PDF order",
          help: "Files are merged from top to bottom.",
          empty: "Add at least two PDF files.",
        },
        split: {
          pagesPerFile: "Pages per file",
          outputDirectory: "Output folder",
        },
        reorder: {
          order: "New page order",
          placeholder: "3, 1, 2, 4-6",
          help: "Include every page exactly once. Ranges are supported.",
        },
        rotate: {
          angle: "Clockwise rotation",
          pages: "Pages",
          placeholder: "1, 3-5",
          help: "Leave empty to rotate every page.",
        },
        errors: {
          page_list_required: "Enter the new page order.",
          invalid_page_list:
            "Use page numbers and ascending ranges such as 1, 3-5.",
          duplicate_pages: "Each page number may appear only once.",
        },
        localNotice: "Runs locally · Originals remain unchanged",
        addToQueue: "Add to queue",
        adding: "Adding…",
        queued: "PDF operation added to the local queue.",
        viewJobs: "View jobs",
      },
      jobs: {
        eyebrow: "Local queue",
        title: "Conversion jobs",
        refresh: "Refresh",
        cancel: "Cancel",
        clearAll: "Clear all",
        clearConfirmation:
          "Remove all completed and cancelled jobs from history? Failed jobs and converted output files will remain.",
        cancelledReason: "Cancelled by user",
        failedReason: "Conversion failed for an unknown reason.",
        sections: {
          clear: "Clear section",
          clearLabel: "Clear {{section}}",
          active: { title: "Active" },
          failed: {
            title: "Failed & Cancelled",
            clearConfirmation:
              "Remove all failed and cancelled jobs from history? Output files will not be deleted.",
          },
          completed: {
            title: "Completed",
            clearConfirmation:
              "Remove all completed jobs from history? Converted output files will not be deleted.",
          },
        },
        deleteEntry: "Delete history entry",
        deleteEntryLabel: "Delete {{name}} from history",
        progress: "Conversion progress",
        pdfMergeDetail: "{{count}} PDF files",
        pdfDetail: "{{source}} · {{operation}}",
        imagesToPdfDetail: "{{count}} image pages",
        pdfToImagesDetail: "{{source}} · {{format}} · {{dpi}} DPI",
        officeToPdfDetail: "{{source}} · Office to PDF",
        pdfToDocxDetail: "{{source}} · PDF to Word",
        status: {
          queued: "Queued",
          running: "Running",
          completed: "Completed",
          failed: "Failed",
          cancelled: "Cancelled",
        },
        empty: {
          title: "No conversion jobs yet",
          description:
            "Image, audio, and PDF conversion jobs will appear here.",
          browserDescription:
            "Job execution is available in the desktop application.",
        },
      },
      files: {
        search: "Search this folder",
        showHidden: "Show hidden",
        emptyFolder: "No matching items in this folder.",
        selectEntry: "Select {{name}}",
        empty: {
          title: "Choose a folder to begin",
          description:
            "Select a local folder to browse and manage its contents.",
          browserDescription:
            "Folder browsing is available in the desktop application.",
        },
        actions: {
          chooseFolder: "Choose folder",
          up: "Parent folder",
          refresh: "Refresh",
          clearSearch: "Clear search",
          newFolder: "New folder",
          rename: "Rename",
          copy: "Copy to…",
          move: "Move to…",
          trash: "Move to trash",
          deletePermanently: "Delete permanently",
        },
        columns: {
          select: "Select",
          name: "Name",
          type: "Type",
          modified: "Modified",
          size: "Size",
        },
        types: {
          folder: "Folder",
          file: "File",
        },
        prompts: {
          folderName: "Name the new folder",
          newName: "Enter a new name",
        },
        confirmations: {
          trash:
            "Move {{count}} selected item(s) to the operating system trash?",
          permanent:
            "Permanently delete {{count}} selected item(s)? This cannot be undone.",
        },
      },
      errors: {
        bootstrap: "The local application capabilities could not be loaded.",
      },
      common: {
        loading: "Loading conversion tools",
      },
    },
  },
  tr: {
    translation: {
      app: {
        name: "Just Convert",
        localWorkspace: "Yerel çalışma alanı",
        localOnly: "Gizli · Yalnızca yerel",
      },
      navigation: {
        primary: "Ana menü",
        files: "Dosyalar",
        convert: "Dönüştür",
        jobs: "İşlemler",
        settings: "Ayarlar",
      },
      preferences: {
        label: "Görünüm tercihleri",
        language: "Dil",
        theme: "Tema",
        system: "Sistem",
        light: "Açık",
        dark: "Koyu",
      },
      settings: {
        eyebrow: "Uygulama ayarları",
        title: "Ayarlar",
        description:
          "Yerel tercihleri ve isteğe bağlı dönüştürme motorlarını yönetin.",
        dependencies: {
          title: "Bağımlılıklar",
          description:
            "Just Convert önce elle ayarlanan yolları, ardından sistemi otomatik olarak denetler.",
          ready: "Hazır",
          notReady: "Kurulum gerekli",
          autoPath: "Otomatik algılanan yol",
          manualPath: "Elle ayarlanan yol",
          notFound: "Otomatik olarak bulunamadı",
          browse: "Göz at…",
          reset: "Otomatik algılamaya dön",
          searchAgain: "Yeniden ara",
          searching: "Aranıyor…",
          invalidPath:
            "Kaydedilen çalıştırılabilir dosya artık yok veya geçersiz. Otomatik yedek: {{fallback}}",
          windowsAppsAlias:
            "Kaydedilen yol gerçek bir Python kurulumu değil, Microsoft Store diğer adıdır. WindowsApps dışındaki bir Python çalıştırılabilir dosyasını seçin.",
          versionCheckFailed:
            "Python 3.10 veya üzeri doğrulanamadı. Elle ayarlanan yol kaydedildi ve yine de kullanılacak.",
          noFallback: "bulunamadı",
          packagesMissing:
            "Python bulundu ancak bu yorumlayıcıda PyMuPDF ve pdf2docx kurulu değil.",
          unavailable: "Bağımlılık şu anda kullanılamıyor.",
        },
      },
      dashboard: {
        privateEyebrow: "Gizlilik odaklı",
        title: "Dosyalarınız, bilgisayarınız.",
        description:
          "Dosyalarınızı yüklemeden göz atın, yönetin ve dönüştürün. Tüm işlemler bu bilgisayarda kalır.",
        browse: "Dosyalara göz at",
      },
      converters: {
        eyebrow: "Yetenekler",
        title: "Dönüştürme araçları",
        imagesToPdf: {
          title: "Görsellerden PDF",
          description:
            "JPG, PNG, WebP ve TIFF görsellerini tek bir yerel PDF dosyasında birleştirin.",
        },
        pdfTools: {
          title: "PDF düzenle",
          description:
            "qpdf ile PDF sayfalarını birleştirin, ayırın, sıralayın ve döndürün.",
        },
        pdfToImages: {
          title: "PDF'den görsellere",
          description:
            "PDF sayfalarını yüksek kaliteli PNG veya JPEG dosyalarına dönüştürün.",
        },
        audio: {
          title: "Ses dönüştür",
          description:
            "FFmpeg ile MP3, WAV, FLAC, AAC, M4A ve OGG biçimleri arasında dönüştürün.",
        },
        officeToPdf: {
          title: "Office'ten PDF'ye",
          description:
            "Yerel LibreOffice ile DOCX, XLSX ve PPTX dosyalarını dönüştürün.",
        },
        pdfToDocx: {
          title: "PDF'den Word'e",
          description:
            "Seçilebilir metin içeren PDF'ler için en iyi sonucu hedefleyen DOCX dönüşümü.",
        },
      },
      status: {
        ready: "Hazır",
        external_required: "Kurulum gerekli",
        planned: "Planlandı",
        bundled: "Dahil",
        external: "Harici",
      },
      batchMode: {
        label: "Kaynak seçim modu",
        file: "Tek dosya",
        folder: "Klasör grubu",
      },
      audio: {
        back: "Tüm dönüştürme araçları",
        title: "Ses dönüştür",
        description:
          "Dahil edilen FFmpeg motoruyla tek bir ses dosyasını veya tam bir klasörü dönüştürün.",
        mode: {
          label: "Ses kaynağı modu",
          file: "Tek dosya",
          folder: "Klasör grubu",
        },
        input: {
          label: "Kaynak dosya",
          help: "MP3, WAV, FLAC, AAC, M4A veya OGG",
          choose: "Ses dosyası seç",
          change: "Değiştir",
        },
        format: {
          label: "Çıktı biçimi",
          help: "MP3, WAV, FLAC, AAC, M4A veya OGG",
        },
        output: {
          label: "Kayıt konumu",
          help: "Mevcut dosyaların üzerine asla yazılmaz.",
          choose: "Çıktı seç",
        },
        folderInput: {
          label: "Kaynak klasör",
          help: "Bu klasördeki desteklenen ses dosyaları dahil edilir; alt klasörler atlanır.",
          choose: "Kaynak klasörü seç",
        },
        folderOutput: {
          label: "Çıktı klasörü",
          help: "Ad çakışmalarında -2, -3 şeklinde ek eklenir. Orijinaller değişmez.",
          choose: "Çıktı klasörünü seç",
        },
        localNotice: "Yerel çalışır · Orijinaller değişmeden kalır",
        convert: "Kuyruğa ekle",
        adding: "Ekleniyor…",
        queued: "Dönüştürme yerel kuyruğa eklendi.",
        batchQueued: "{{count}} ses dosyası yerel kuyruğa eklendi.",
        viewJobs: "İşlemleri görüntüle",
      },
      images: {
        back: "Tüm dönüştürme araçları",
        engine: "Yerel",
        title: "Görsellerden PDF",
        description:
          "Yerel görselleri seçtiğiniz sırayla tek bir PDF dosyasında birleştirin.",
        order: "Görsel sırası",
        orderHelp: "Her görsel yukarıdan aşağıya bir PDF sayfası olur.",
        add: "Görsel ekle",
        empty: "Bir veya daha fazla JPG, PNG, WebP ya da TIFF görseli ekleyin.",
        moveUp: "{{name}} dosyasını yukarı taşı",
        moveDown: "{{name}} dosyasını aşağı taşı",
        remove: "{{name}} dosyasını kaldır",
        pageLayout: "Otomatik A4 düzeni",
        pageLayoutHelp:
          "Her görsel için dikey veya yatay yön seçilir. En-boy oranı küçük bir kenar boşluğuyla korunur.",
        output: "Çıktı PDF'si",
        outputHelp: "Mevcut dosyaların üzerine asla yazılmaz.",
        chooseOutput: "Çıktı seç",
        localNotice: "Yerel çalışır · Orijinaller değişmeden kalır",
        addToQueue: "Kuyruğa ekle",
        adding: "Ekleniyor…",
        queued: "Görsellerden PDF'ye dönüştürme yerel kuyruğa eklendi.",
        folderInput: "Kaynak klasör",
        folderInputHelp:
          "Üst düzeydeki desteklenen tüm görseller dosya adı sırasıyla dahil edilir; alt klasörler atlanır.",
        changeFolder: "Klasörü değiştir",
        chooseSourceFolder: "Kaynak klasörü seç",
        folderMode: "PDF çıktı düzeni",
        folderModeHelp:
          "Klasördeki görsellerin nasıl PDF'ye dönüştürüleceğini seçin.",
        combine: "Tüm görselleri tek bir PDF'de birleştir",
        combineHelp:
          "Dosya adı sırasına göre çok sayfalı tek bir PDF oluşturur.",
        separate: "Her görsel için ayrı bir PDF",
        separateHelp: "Her görsel için ayrı, tek sayfalı bir PDF oluşturur.",
        folderOutput: "Çıktı klasörü",
        folderOutputHelpCombine:
          "Tek bir birleşik PDF oluşturur; ad çakışmalarına -2, -3 şeklinde ek ekler.",
        folderOutputHelpSeparate:
          "Her görselin temel dosya adını kullanır; çakışmalara -2, -3 şeklinde ek ekler.",
        chooseOutputFolder: "Çıktı klasörünü seç",
        batchQueuedCombine:
          "Klasördeki görseller tek bir birleşik PDF işi olarak eklendi.",
        batchQueuedSeparate:
          "{{count}} ayrı görselden PDF'ye işi kuyruğa eklendi.",
        viewJobs: "İşlemleri görüntüle",
      },
      pdfImages: {
        back: "Tüm dönüştürme araçları",
        title: "PDF'den görsellere",
        description:
          "Her PDF sayfasını ayrı bir yerel PNG veya JPEG görseline dönüştürün.",
        input: "Kaynak PDF",
        inputHelp: "Sayfa sayısını incelemek için bir PDF seçin.",
        pageCount: "{{count}} sayfa",
        choosePdf: "PDF seç",
        format: "Görsel biçimi",
        formatHelp: "PNG kayıpsızdır; JPEG daha küçük dosyalar oluşturur.",
        resolution: "Çözünürlük",
        resolutionHelp:
          "Yüksek çözünürlük daha fazla bellek ve disk alanı kullanır.",
        standard: "Standart · 150 DPI",
        high: "Yüksek · 300 DPI",
        output: "Çıktı klasörü",
        outputHelp:
          "Adlar biçim ve kaliteyi içerir; çakışmalar -2, -3 şeklinde yeniden adlandırılır.",
        chooseFolder: "Klasör seç",
        localNotice: "Yerel çalışır · Her sayfa için bir görsel",
        addToQueue: "Kuyruğa ekle",
        adding: "Ekleniyor…",
        queued: "PDF'den görsellere dönüştürme yerel kuyruğa eklendi.",
        folderInput: "Kaynak klasör",
        folderInputHelp:
          "Üst düzeydeki tüm PDF'ler dahil edilir; alt klasörler ve desteklenmeyen dosyalar atlanır.",
        chooseSourceFolder: "Kaynak klasörü seç",
        folderOutputHelp:
          "Her PDF için kaynak dosya adına sahip, çakışmaya karşı güvenli bir çıktı alt klasörü oluşturulur.",
        batchQueued: "{{count}} PDF dosyası yerel kuyruğa eklendi.",
        viewJobs: "İşlemleri görüntüle",
      },
      office: {
        back: "Tüm dönüştürme araçları",
        title: "Office'ten PDF'ye",
        description:
          "DOCX, XLSX veya PPTX dosyalarını yerel LibreOffice kurulumunuzla dönüştürün.",
        input: "Office belgesi",
        inputHelp: "DOCX, XLSX veya PPTX",
        chooseFile: "Belge seç",
        output: "Çıktı PDF'si",
        outputHelp: "Mevcut dosyaların üzerine asla yazılmaz.",
        chooseOutput: "Çıktı seç",
        localNotice: "Yerel çalışır · Yalıtılmış LibreOffice profili kullanır",
        addToQueue: "Kuyruğa ekle",
        adding: "Ekleniyor…",
        queued: "Office'ten PDF'ye dönüştürme yerel kuyruğa eklendi.",
        folderInput: "Kaynak klasör",
        folderInputHelp:
          "Üst düzey DOCX, XLSX ve PPTX dosyaları dahil edilir; diğer dosyalar ve alt klasörler atlanır.",
        chooseSourceFolder: "Kaynak klasörü seç",
        folderOutputHelp:
          "Her belge için bir PDF oluşturulur; ad çakışmalarına -2, -3 şeklinde ek eklenir.",
        chooseOutputFolder: "Çıktı klasörünü seç",
        batchQueued: "{{count}} Office belgesi yerel kuyruğa eklendi.",
        viewJobs: "İşlemleri görüntüle",
        missing: {
          title: "LibreOffice gerekli",
          description:
            "LibreOffice'i kurun, ardından algılanması için Just Convert'i yeniden başlatın.",
          download: "LibreOffice'i indir",
          setPath: "Yolu elle ayarla",
        },
      },
      pdfDocx: {
        back: "Tüm dönüştürme araçları",
        title: "PDF'den Word'e",
        description:
          "Seçilebilir metin içeren bir PDF'yi yerel Python kurulumunuzla düzenlenebilir DOCX'e dönüştürün.",
        input: "Kaynak PDF",
        inputHelp: "Yalnızca seçilebilir metin içeren dijital PDF'ler",
        chooseFile: "PDF seç",
        output: "Çıktı Word belgesi",
        outputHelp: "Mevcut dosyaların üzerine asla yazılmaz.",
        chooseOutput: "Çıktı seç",
        localNotice: "Yerel çalışır · Orijinaller değişmeden kalır",
        addToQueue: "Kuyruğa ekle",
        adding: "Ekleniyor…",
        queued: "PDF'den Word'e dönüştürme yerel kuyruğa eklendi.",
        folderInput: "Kaynak klasör",
        folderInputHelp:
          "Üst düzey PDF'ler kuyruğa eklenir; desteklenmeyen dosyalar ve alt klasörler atlanır. Taranmış PDF'ler nedenleriyle birlikte başarısız iş olarak görünür.",
        chooseSourceFolder: "Kaynak klasörü seç",
        folderOutputHelp:
          "Her PDF için bir DOCX oluşturulur; ad çakışmalarına -2, -3 şeklinde ek eklenir.",
        chooseOutputFolder: "Çıktı klasörünü seç",
        batchQueued: "{{count}} PDF dosyası yerel kuyruğa eklendi.",
        viewJobs: "İşlemleri görüntüle",
        quality: {
          title: "En iyi çaba ile düzen dönüştürme",
          description:
            "Metinler, tablolar ve basit düzenler genellikle makul biçimde dönüştürülür. Karmaşık sütunlar, grafikler ve hassas biçimlendirme farklı olabilir. Yalnızca görsel içeren taranmış PDF'ler reddedilir; OCR daha sonra eklenecektir.",
        },
        missing: {
          title: "Python dönüştürme bağımlılıkları gerekli",
          description:
            "Python 3.10 veya üstünü kurun, ardından aşağıdaki komutla PyMuPDF ve pdf2docx paketlerini yükleyin. Kurulumdan sonra Just Convert'i yeniden başlatın.",
          python: "Python'ı indir",
          copyCommand: "Kurulum komutunu kopyala",
          copy: "Kopyala",
          copied: "Kopyalandı",
          copyFailed: "Kopyalanamadı",
          setPath: "Yolu elle ayarla",
        },
      },
      pdf: {
        back: "Tüm dönüştürme araçları",
        title: "PDF düzenle",
        description:
          "PDF sayfalarını qpdf ile yerel olarak birleştirin, ayırın, sıralayın veya döndürün.",
        modeLabel: "PDF işlemi",
        modes: {
          merge: "Birleştir",
          split: "Ayır",
          reorder: "Sırala",
          rotate: "Döndür",
        },
        choosePdfs: "PDF ekle",
        choosePdf: "PDF seç",
        chooseFolder: "Klasör seç",
        chooseOutput: "Çıktı seç",
        source: "Kaynak PDF",
        sourceHelp: "Sayfa sayısını incelemek için bir PDF seçin.",
        pageCount: "{{count}} sayfa",
        output: "Çıktı PDF'si",
        outputHelp: "Mevcut dosyaların üzerine asla yazılmaz.",
        moveUp: "{{name}} dosyasını yukarı taşı",
        moveDown: "{{name}} dosyasını aşağı taşı",
        remove: "{{name}} dosyasını kaldır",
        merge: {
          files: "PDF sırası",
          help: "Dosyalar yukarıdan aşağıya birleştirilir.",
          empty: "En az iki PDF dosyası ekleyin.",
        },
        split: {
          pagesPerFile: "Dosya başına sayfa",
          outputDirectory: "Çıktı klasörü",
        },
        reorder: {
          order: "Yeni sayfa sırası",
          placeholder: "3, 1, 2, 4-6",
          help: "Her sayfayı tam bir kez ekleyin. Aralıklar desteklenir.",
        },
        rotate: {
          angle: "Saat yönünde döndürme",
          pages: "Sayfalar",
          placeholder: "1, 3-5",
          help: "Tüm sayfaları döndürmek için boş bırakın.",
        },
        errors: {
          page_list_required: "Yeni sayfa sırasını girin.",
          invalid_page_list:
            "1, 3-5 gibi sayfa numaraları ve artan aralıklar kullanın.",
          duplicate_pages:
            "Her sayfa numarası yalnızca bir kez kullanılabilir.",
        },
        localNotice: "Yerel çalışır · Orijinaller değişmeden kalır",
        addToQueue: "Kuyruğa ekle",
        adding: "Ekleniyor…",
        queued: "PDF işlemi yerel kuyruğa eklendi.",
        viewJobs: "İşlemleri görüntüle",
      },
      jobs: {
        eyebrow: "Yerel kuyruk",
        title: "Dönüştürme işlemleri",
        refresh: "Yenile",
        cancel: "İptal et",
        clearAll: "Tümünü temizle",
        clearConfirmation:
          "Tamamlanan ve iptal edilen tüm işlemler geçmişten kaldırılsın mı? Başarısız işlemler ve dönüştürülen çıktı dosyaları korunur.",
        cancelledReason: "Kullanıcı tarafından iptal edildi",
        failedReason: "Dönüştürme bilinmeyen bir nedenle başarısız oldu.",
        sections: {
          clear: "Bölümü temizle",
          clearLabel: "{{section}} bölümünü temizle",
          active: { title: "Devam Eden İşlemler" },
          failed: {
            title: "Başarısız ve İptal Edilenler",
            clearConfirmation:
              "Tüm başarısız ve iptal edilen işlemler geçmişten kaldırılsın mı? Çıktı dosyaları silinmez.",
          },
          completed: {
            title: "Tamamlananlar",
            clearConfirmation:
              "Tamamlanan tüm işlemler geçmişten kaldırılsın mı? Dönüştürülen çıktı dosyaları silinmez.",
          },
        },
        deleteEntry: "Geçmiş kaydını sil",
        deleteEntryLabel: "{{name}} kaydını geçmişten sil",
        progress: "Dönüştürme ilerlemesi",
        pdfMergeDetail: "{{count}} PDF dosyası",
        pdfDetail: "{{source}} · {{operation}}",
        imagesToPdfDetail: "{{count}} görsel sayfası",
        pdfToImagesDetail: "{{source}} · {{format}} · {{dpi}} DPI",
        officeToPdfDetail: "{{source}} · Office'ten PDF'ye",
        pdfToDocxDetail: "{{source}} · PDF'den Word'e",
        status: {
          queued: "Kuyrukta",
          running: "Çalışıyor",
          completed: "Tamamlandı",
          failed: "Başarısız",
          cancelled: "İptal edildi",
        },
        empty: {
          title: "Henüz dönüştürme işlemi yok",
          description:
            "Görsel, ses ve PDF dönüştürme işlemleri burada görünür.",
          browserDescription:
            "İşlem yürütme masaüstü uygulamasında kullanılabilir.",
        },
      },
      files: {
        search: "Bu klasörde ara",
        showHidden: "Gizlileri göster",
        emptyFolder: "Bu klasörde eşleşen öğe yok.",
        selectEntry: "{{name}} öğesini seç",
        empty: {
          title: "Başlamak için bir klasör seçin",
          description:
            "İçeriğine göz atmak ve yönetmek için yerel bir klasör seçin.",
          browserDescription:
            "Klasörlere göz atma özelliği masaüstü uygulamasında kullanılabilir.",
        },
        actions: {
          chooseFolder: "Klasör seç",
          up: "Üst klasör",
          refresh: "Yenile",
          clearSearch: "Aramayı temizle",
          newFolder: "Yeni klasör",
          rename: "Yeniden adlandır",
          copy: "Şuraya kopyala…",
          move: "Şuraya taşı…",
          trash: "Çöp kutusuna taşı",
          deletePermanently: "Kalıcı olarak sil",
        },
        columns: {
          select: "Seç",
          name: "Ad",
          type: "Tür",
          modified: "Değiştirilme",
          size: "Boyut",
        },
        types: {
          folder: "Klasör",
          file: "Dosya",
        },
        prompts: {
          folderName: "Yeni klasörün adı",
          newName: "Yeni bir ad girin",
        },
        confirmations: {
          trash:
            "Seçilen {{count}} öğe işletim sistemi çöp kutusuna taşınsın mı?",
          permanent:
            "Seçilen {{count}} öğe kalıcı olarak silinsin mi? Bu işlem geri alınamaz.",
        },
      },
      errors: {
        bootstrap: "Yerel uygulama yetenekleri yüklenemedi.",
      },
      common: {
        loading: "Dönüştürme araçları yükleniyor",
      },
    },
  },
} as const;
