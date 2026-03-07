import * as pdfjs from 'pdfjs-dist';

// MATCH THIS TO THE VERSION IN package.json
const PDFJS_VERSION = '5.5.207';
pdfjs.GlobalWorkerOptions.workerSrc = `https://unpkg.com/pdfjs-dist@${PDFJS_VERSION}/build/pdf.worker.min.mjs`;

/**
 * Extracts all text from a PDF file.
 */
export async function extractTextFromPDF(file: File): Promise<string> {
  try {
    const arrayBuffer = await file.arrayBuffer();
    
    // Validate we're sending a real array buffer
    if (arrayBuffer.byteLength === 0) {
      throw new Error('PDF file is empty.');
    }

    const loadingTask = pdfjs.getDocument({ 
      data: arrayBuffer,
      useSystemFonts: true,
      // Increase verbosity for logging in the console
      verbosity: 1, 
    });
    
    const pdf = await loadingTask.promise;
    let fullText = '';

    for (let i = 1; i <= pdf.numPages; i++) {
      const page = await pdf.getPage(i);
      const textContent = await page.getTextContent();
      
      const pageText = textContent.items
        .map((item: any) => item.str)
        .join(' ');
      
      fullText += pageText + ' ';
    }

    const cleanedText = fullText.replace(/\s+/g, ' ').trim();
    if (!cleanedText) {
      throw new Error('Could not find readable text (PDF might be an image/scan).');
    }

    return cleanedText;
  } catch (error) {
    // This logs to the BROWSER console (F12)
    console.error('Detailed PDF Extraction Error:', error);
    throw error;
  }
}
