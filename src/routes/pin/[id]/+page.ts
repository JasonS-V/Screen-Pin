export function load({ params }: { params: { id: string } }) {
  return {
    pinId: params.id,
  };
}
