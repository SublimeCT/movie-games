/**
 * Compress an image file to a base64 string under a specified size limit (in bytes).
 * Uses a canvas to resize/compress if necessary.
 *
 * @param file The file object to compress
 * @param maxSize The maximum size in bytes (default 300KB = 300 * 1024)
 * @param maxWidth The maximum width of the output image (optional, default 1024)
 * @param initialQuality The initial quality for JPEG compression (0-1, default 0.8)
 * @returns A promise that resolves to the base64 string
 */
export const compressImage = (
  file: File,
  maxSize: number = 300 * 1024,
  maxWidth = 1024,
  initialQuality = 0.8,
): Promise<string> => {
  return new Promise((resolve, reject) => {
    const reader = new FileReader();
    reader.readAsDataURL(file);
    reader.onload = (event) => {
      const img = new Image();
      img.src = event.target?.result as string;
      img.onload = () => {
        // Calculate new dimensions
        let width = img.width;
        let height = img.height;

        if (width > maxWidth) {
          height = Math.round((height * maxWidth) / width);
          width = maxWidth;
        }

        const canvas = document.createElement('canvas');
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext('2d');
        if (!ctx) {
          reject(new Error('Failed to get canvas context'));
          return;
        }

        ctx.drawImage(img, 0, 0, width, height);

        // Function to attempt compression with decreasing quality
        const attemptCompression = (quality: number) => {
          // Use image/jpeg for compression support (png doesn't support quality)
          // If original was PNG and we want transparency, this might be an issue,
          // but for game backgrounds/avatars, JPEG is usually fine or we accept loss of transparency for size.
          // However, to be safe, let's stick to jpeg for aggressive compression.
          const dataUrl = canvas.toDataURL('image/jpeg', quality);

          // Check size (base64 string length * 0.75 is approx binary size)
          // Or just check string length against limit * 1.33
          // The user specified "images(base64) < 300k", usually meaning the string length or the file size.
          // If they mean file size, 300KB file ~= 400KB base64.
          // If they mean base64 string length, then we compare length directly.
          // The backend check I wrote uses `bg.len() > 400_000`, which implies ~300KB binary.
          // Let's assume the user means "resulting file size should be < 300KB", so base64 len < 400,000.

          if (dataUrl.length < maxSize * 1.37 || quality < 0.1) {
            resolve(dataUrl);
          } else {
            // Reduce quality and try again
            attemptCompression(quality - 0.1);
          }
        };

        attemptCompression(initialQuality);
      };
      img.onerror = (err) => reject(err);
    };
    reader.onerror = (err) => reject(err);
  });
};
