import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/primitives";
import { listen } from "@tauri-apps/api/event";
import { getCurrent } from "@tauri-apps/api/window";
import { XIcon, MinusIcon, SquareIcon } from "lucide-react";
import { Button } from "./components/ui/button";

const FPSCounter = () => {

  const [fps, setFps] = useState(0)

  // @ts-expect-error
  useEffect(() => {

    let lastTime = Date.now(), nbFrames = 0;

    const unsub = listen("image", (event) => {
      // setPayload(event.payload as any);
      const currentTime = Date.now()
      nbFrames++
      if(currentTime - lastTime >= 1000) {
        setFps(nbFrames)
        nbFrames = 0
        lastTime = Date.now()
      }
    });

    return () => unsub.then((u) => u());
  }, []);

  return <div>{fps}</div>
}

const ImageCanvas = () => {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  // @ts-expect-error
  useEffect(() => {
    const unsub = listen("image", (event) => {
      const {width, height, image} = event.payload as {
        image: number[];
        width: number;
        height: number;
      } 

      if (canvasRef.current) {
        const canvas = canvasRef.current

        canvas.width = width
        canvas.height = height

        const ctx = canvas.getContext("2d");
  
        if (!ctx) {
          return;
        }
  
        console.log({ length: image.length, total: width * height * 4 });
  
        const imageData = ctx.createImageData(width, height);
        imageData.data.set(image);
        ctx.putImageData(imageData, 0, 0);
      }
    });

    return () => unsub.then((u) => u());
  }, []);

  return (
    <canvas
      ref={canvasRef}
      width={0}
      height={0}
      className="w-full h-full"
    />
  );
};

const appWindow = getCurrent();

function App() {
  async function startCapture() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    invoke("start_capture");
  }

  async function stopCapture() {
    // Learn more about Tauri commands at https://tauri.app/v1/guides/features/command
    invoke("stop_capture");
  }

  return (
    <div className="bg-red-800 p-2 h-full">
      <main className="bg-background h-full rounded">
        <header>
          <div
            data-tauri-drag-region
            className="h-8 bg-transparent flex justify-end items-center inset-x-0 top-0"
          >
            <button
              className="inline-flex justify-center items-center w-10 h-8 hover:bg-slate-800"
              id="titlebar-minimize"
              onClick={() => appWindow.minimize()}
            >
              <MinusIcon className="h-3 w-3" />
            </button>
            <button
              className="inline-flex justify-center items-center w-10 h-8 hover:bg-slate-800"
              id="titlebar-maximize"
              onClick={() => appWindow.toggleMaximize()}
            >
              <SquareIcon className="h-2.5 w-2.5" />
            </button>
            <button
              className="inline-flex justify-center items-center w-10 h-8 hover:bg-destructive"
              onClick={() => appWindow.close()}
            >
              <XIcon className="h-3 w-3" />
            </button>
            
          </div>
        </header>

        <div className="p-8">
          <h1>Welcome Hugo!</h1>

          <p>Click on the Tauri, Vite, and React logos to learn more.</p>

            <Button type="submit" onClick={startCapture}>Start</Button>
            <Button type="submit" onClick={stopCapture}>Stop</Button>
            <FPSCounter />


          <ImageCanvas
          />
        </div>
      </main>
    </div>
  );
}

export default App;
