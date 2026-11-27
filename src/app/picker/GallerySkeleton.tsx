import { text } from "../i18n/text";

const PLACEHOLDERS = [0, 1, 2, 3];

export function GallerySkeleton() {
  return (
    <section className="group">
      <h2 className="group-label">
        <span className="label">{text.groupLoading}</span>
      </h2>
      <div className="gallery" aria-busy="true">
        {PLACEHOLDERS.map((index) => (
          <div key={index} className="skeleton-tile">
            <div className="skeleton" data-shape="thumb" />
            <div className="skeleton" data-shape="name" />
          </div>
        ))}
      </div>
    </section>
  );
}
