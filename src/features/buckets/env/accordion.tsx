import { AccordionSection } from "@/shared/ui/accordion-section";
import { BucketEnvCredentials } from "@/features/buckets/env/creds";

interface BucketEnvAccordionProps {
  bucketId: string;
  cachedToken?: string | null;
  onTokenCached?: (token: string) => void;
}

export function BucketEnvAccordion({
  bucketId,
  cachedToken,
  onTokenCached,
}: BucketEnvAccordionProps) {
  return (
    <AccordionSection
      title="Project .env"
      description="ARGUS_BUCKET_ID and ARGUS_BUCKET_TOKEN for load_env in your project."
    >
      <BucketEnvCredentials
        embedded
        bucketId={bucketId}
        cachedToken={cachedToken}
        onTokenCached={onTokenCached}
      />
    </AccordionSection>
  );
}
