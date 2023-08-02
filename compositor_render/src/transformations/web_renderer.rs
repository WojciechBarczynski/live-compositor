use std::{sync::Arc, thread, time::Duration};

use crate::renderer::{
    texture::{NodeTexture, RGBATexture},
    RenderCtx,
};
pub mod electron;
mod electron_api;

use compositor_common::{
    scene::{NodeId, Resolution},
    transformation::WebRendererTransformationParams,
};
use image::ImageError;
use log::error;
use serde::{Deserialize, Serialize};

use self::electron_api::ElectronApiError;

pub type ElectronInstance = self::electron::ElectronInstance;

#[derive(Debug, Serialize, Deserialize)]
pub struct SessionId(pub Arc<str>);

pub struct WebRenderer {
    session_id: SessionId,
    #[allow(dead_code)]
    params: WebRendererTransformationParams,
}

impl WebRenderer {
    pub fn new(
        ctx: &RenderCtx,
        params: WebRendererTransformationParams,
    ) -> Result<Self, WebRendererNewError> {
        // TODO: wait electron api by checking tcp connection on that port
        thread::sleep(Duration::from_secs(5));
        let session_id = ctx
            .electron
            .client
            .new_session(&params.url, params.resolution)?;

        Ok(Self { session_id, params })
    }

    pub fn render(
        &self,
        ctx: &RenderCtx,
        _sources: &[(&NodeId, &NodeTexture)],
        target: &NodeTexture,
    ) -> Result<(), WebRendererRenderError> {
        let frame = ctx.electron.client.get_frame(&self.session_id)?;
        if !frame.is_empty() {
            Self::write_texture(ctx, &frame, &target.rgba_texture())?;
        }

        Ok(())
    }

    fn write_texture(
        ctx: &RenderCtx,
        data: &[u8],
        target: &RGBATexture,
    ) -> Result<(), WebRendererRenderError> {
        let size = target.size();
        let img = image::load_from_memory(data)?;

        if img.width() != size.width || img.height() != size.height {
            return Err(WebRendererRenderError::InvalidFrameResolution {
                expected: Resolution {
                    width: size.width as usize,
                    height: size.height as usize,
                },
                received: Resolution {
                    width: img.width() as usize,
                    height: img.height() as usize,
                },
            });
        }

        target.upload(ctx.wgpu_ctx, &img.to_rgba8());
        ctx.wgpu_ctx.queue.submit([]);
        Ok(())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum WebRendererNewError {
    #[error("failed to create new web renderer session")]
    SessionCreationError(#[from] ElectronApiError),
}

#[derive(Debug, thiserror::Error)]
pub enum WebRendererRenderError {
    #[error("expected string param")]
    InvalidParam,

    #[error("communication with web renderer failed")]
    HttpError(#[from] ElectronApiError),

    #[error("failed to decode image data")]
    ImageDecodeError(#[from] ImageError),

    #[error("web renderer sent frame with invalid resolution. Expected {expected:?}, received {received:?}")]
    InvalidFrameResolution {
        expected: Resolution,
        received: Resolution,
    },
}
