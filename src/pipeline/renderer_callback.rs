use crossbeam_channel::{Receiver, Sender};
use eframe::egui_wgpu::CallbackTrait;
use once_cell::sync::OnceCell;
use wgpu::CommandBuffer;

static SENDER: OnceCell<Sender<CommandBuffer>> = OnceCell::new();
static RECEIVER: OnceCell<Receiver<CommandBuffer>> = OnceCell::new();

pub struct RendererCallback;

impl RendererCallback {
    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn init() {
        let (tx, rx) = crossbeam_channel::bounded(16);
        SENDER.set(tx).expect("Could not set SENDER");
        RECEIVER.set(rx).expect("Could not set RECEIVER");
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    pub fn add(buffer: CommandBuffer) {
        SENDER
            .get()
            .expect("SENDER is not yet initialized")
            .send(buffer)
            .expect("Could not send command buffer");
    }
}

impl CallbackTrait for RendererCallback {
    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        _render_pass: &mut wgpu::RenderPass<'static>,
        _callback_resources: &eframe::egui_wgpu::CallbackResources,
    ) {
    }

    #[cfg_attr(feature = "profiling", profiling::function)]
    fn prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _screen_descriptor: &eframe::egui_wgpu::ScreenDescriptor,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let mut buffers = Vec::with_capacity(16);

        while let Ok(buffer) = RECEIVER
            .get()
            .expect("RECEIVER is not yet initialized")
            .try_recv()
        {
            buffers.push(buffer);
        }

        buffers
    }

    fn finish_prepare(
        &self,
        _device: &wgpu::Device,
        _queue: &wgpu::Queue,
        _egui_encoder: &mut wgpu::CommandEncoder,
        _callback_resources: &mut eframe::egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        Vec::new()
    }
}
